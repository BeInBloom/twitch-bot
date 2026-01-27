use crate::types::{TwitchEvent, TwitchRole, TwitchUser};

pub fn parse_irc_messages(raw: &str) -> Vec<TwitchEvent> {
    raw.split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .filter(|line| !line.is_empty())
        .filter_map(parse_line)
        .collect()
}

struct IrcMessage<'a> {
    tags: &'a str,
    command: &'a str,
    params: &'a str,
}

fn parse_irc_structure(line: &str) -> Option<IrcMessage<'_>> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let (tags, rest) = if let Some(stripped) = line.strip_prefix('@') {
        stripped.split_once(' ')?
    } else {
        ("", line)
    };

    let rest = if let Some(stripped) = rest.strip_prefix(':') {
        stripped.split_once(' ').map(|(_, r)| r)?
    } else {
        rest
    };

    let (command, params) = rest.split_once(' ')?;

    Some(IrcMessage {
        tags,
        command,
        params,
    })
}

fn parse_line(line: &str) -> Option<TwitchEvent> {
    let msg = parse_irc_structure(line)?;

    match msg.command {
        "PRIVMSG" => parse_privmsg(msg.tags, msg.params),
        _ => None,
    }
}

fn parse_privmsg(tags: &str, params: &str) -> Option<TwitchEvent> {
    let channel_and_text = params.split_once(" :")?;
    let channel = channel_and_text.0.strip_prefix('#').map(str::to_string);
    let text = channel_and_text.1.to_string();

    let meta = parse_tags(tags);

    Some(TwitchEvent::ChatMessage {
        user: TwitchUser {
            id: meta.user_id.to_string(),
            display_name: meta.display_name.to_string(),
            role: meta.role,
        },
        channel,
        text,
    })
}

struct UserMeta<'a> {
    user_id: &'a str,
    display_name: &'a str,
    role: TwitchRole,
}

fn parse_badges(badges: &str) -> TwitchRole {
    let mut role = TwitchRole::empty();

    for badge in badges.split(',') {
        match badge {
            _ if badge.starts_with("broadcaster/") => role.add(TwitchRole::BROADCASTER),
            _ if badge.starts_with("vip/") => role.add(TwitchRole::VIP),
            _ if badge.starts_with("subscriber/") => role.add(TwitchRole::SUBSCRIBER),
            _ => {}
        }
    }

    role
}

fn parse_tags(tags: &str) -> UserMeta<'_> {
    if tags.is_empty() {
        return UserMeta {
            user_id: "0",
            display_name: "anon",
            role: TwitchRole::empty(),
        };
    }

    let mut user_id = "0";
    let mut display_name: Option<&str> = None;
    let mut login: Option<&str> = None;
    let mut role = TwitchRole::empty();

    for pair in tags.split(';') {
        let Some((key, val)) = pair.split_once('=') else {
            continue;
        };

        match key {
            "user-id" => user_id = val,
            "display-name" if !val.is_empty() => display_name = Some(val),
            "login" => login = Some(val),
            "mod" if val == "1" => role.add(TwitchRole::MODERATOR),
            "subscriber" if val == "1" => role.add(TwitchRole::SUBSCRIBER),
            "badges" => {
                let badge_role = parse_badges(val);
                role.merge(badge_role);
            }
            _ => {}
        }
    }

    UserMeta {
        user_id,
        display_name: display_name.or(login).unwrap_or("anon"),
        role,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_one(raw: &str) -> TwitchEvent {
        let events = parse_irc_messages(raw);
        assert_eq!(
            events.len(),
            1,
            "Expected exactly 1 event, got {}",
            events.len()
        );
        events.into_iter().next().unwrap()
    }

    fn role(flag: u8) -> TwitchRole {
        let mut r = TwitchRole::empty();
        r.add(flag);
        r
    }

    fn assert_chat_message(
        event: &TwitchEvent,
        expected_id: &str,
        expected_name: &str,
        expected_role: TwitchRole,
        expected_text: &str,
    ) {
        match event {
            TwitchEvent::ChatMessage { user, text, .. } => {
                assert_eq!(user.id, expected_id);
                assert_eq!(user.display_name, expected_name);
                assert_eq!(
                    user.role, expected_role,
                    "Expected role {:?}, got {:?}",
                    expected_role, user.role
                );
                assert_eq!(text, expected_text);
            }
            _ => panic!("Expected ChatMessage, got {:?}", event),
        }
    }

    #[test]
    fn test_parse_privmsg() {
        let raw = "@badge-info=;badges=broadcaster/1;display-name=TestUser;mod=0;user-id=12345 :testuser!testuser@testuser.tmi.twitch.tv PRIVMSG #channel :Hello world!";
        let event = parse_one(raw);
        assert_chat_message(
            &event,
            "12345",
            "TestUser",
            role(TwitchRole::BROADCASTER),
            "Hello world!",
        );
    }

    #[test]
    fn test_parse_multiline() {
        let raw = "@user-id=1;display-name=User1 :u1 PRIVMSG #ch :msg1\r\n@user-id=2;display-name=User2 :u2 PRIVMSG #ch :msg2";
        let events = parse_irc_messages(raw);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_parse_multiline_unix_newlines() {
        let raw = "@user-id=1;display-name=User1 :u1 PRIVMSG #ch :msg1\n@user-id=2;display-name=User2 :u2 PRIVMSG #ch :msg2";
        let events = parse_irc_messages(raw);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_parse_broadcaster() {
        let raw = "@badges=broadcaster/1;display-name=Streamer;user-id=1 :s PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(&event, "1", "Streamer", role(TwitchRole::BROADCASTER), "hi");
    }

    #[test]
    fn test_parse_mod() {
        let raw = "@mod=1;display-name=ModUser;user-id=2 :m PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(&event, "2", "ModUser", role(TwitchRole::MODERATOR), "hi");
    }

    #[test]
    fn test_parse_vip() {
        let raw = "@badges=vip/1;display-name=VipUser;user-id=3 :v PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(&event, "3", "VipUser", role(TwitchRole::VIP), "hi");
    }

    #[test]
    fn test_parse_subscriber() {
        let raw = "@subscriber=1;display-name=SubUser;user-id=4 :s PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(&event, "4", "SubUser", role(TwitchRole::SUBSCRIBER), "hi");
    }

    #[test]
    fn test_parse_subscriber_badge() {
        let raw = "@badges=subscriber/12;display-name=SubUser;user-id=5 :s PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(&event, "5", "SubUser", role(TwitchRole::SUBSCRIBER), "hi");
    }

    #[test]
    fn test_parse_pleb() {
        let raw = "@display-name=PlebUser;user-id=6 :p PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(&event, "6", "PlebUser", TwitchRole::empty(), "hi");
    }

    #[test]
    fn test_role_priority_broadcaster_over_mod() {
        let raw = "@badges=broadcaster/1;mod=1;display-name=Test;user-id=1 :t PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(
            &event,
            "1",
            "Test",
            role(TwitchRole::BROADCASTER | TwitchRole::MODERATOR),
            "hi",
        );
    }

    #[test]
    fn test_role_priority_mod_over_vip() {
        let raw = "@badges=vip/1;mod=1;display-name=Test;user-id=1 :t PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(
            &event,
            "1",
            "Test",
            role(TwitchRole::MODERATOR | TwitchRole::VIP),
            "hi",
        );
    }

    #[test]
    fn test_empty_display_name_fallback_to_login() {
        let raw = "@display-name=;login=mylogin;user-id=123 :user PRIVMSG #ch :test";
        let event = parse_one(raw);
        assert_chat_message(&event, "123", "mylogin", TwitchRole::empty(), "test");
    }

    #[test]
    fn test_missing_display_name_fallback_to_login() {
        let raw = "@login=fallbacklogin;user-id=456 :user PRIVMSG #ch :test";
        let event = parse_one(raw);
        assert_chat_message(&event, "456", "fallbacklogin", TwitchRole::empty(), "test");
    }

    #[test]
    fn test_no_name_tags_fallback_to_anon() {
        let raw = "@user-id=789 :user PRIVMSG #ch :test";
        let event = parse_one(raw);
        assert_chat_message(&event, "789", "anon", TwitchRole::empty(), "test");
    }

    #[test]
    fn test_empty_input() {
        let events = parse_irc_messages("");
        assert!(events.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let events = parse_irc_messages("   \n\r\n   ");
        assert!(events.is_empty());
    }

    #[test]
    fn test_non_privmsg_ignored() {
        let events = parse_irc_messages("PING :tmi.twitch.tv");
        assert!(events.is_empty());

        let events = parse_irc_messages(":user!user@user.tmi.twitch.tv JOIN #channel");
        assert!(events.is_empty());

        let events = parse_irc_messages(
            "@msg-id=slow_off :tmi.twitch.tv NOTICE #channel :This room is no longer in slow mode.",
        );
        assert!(events.is_empty());
    }

    #[test]
    fn test_malformed_no_command() {
        let events = parse_irc_messages("@tags-only-no-rest");
        assert!(events.is_empty());
    }

    #[test]
    fn test_malformed_no_message() {
        let events = parse_irc_messages("@user-id=1 :user PRIVMSG #channel");
        assert!(events.is_empty());
    }

    #[test]
    fn test_message_with_colon() {
        let raw = "@user-id=1;display-name=Test :t PRIVMSG #ch :hello: world: test";
        let event = parse_one(raw);
        assert_chat_message(
            &event,
            "1",
            "Test",
            TwitchRole::empty(),
            "hello: world: test",
        );
    }

    #[test]
    fn test_message_with_url() {
        let raw = "@user-id=1;display-name=Test :t PRIVMSG #ch :check https://example.com/page";
        let event = parse_one(raw);
        assert_chat_message(
            &event,
            "1",
            "Test",
            TwitchRole::empty(),
            "check https://example.com/page",
        );
    }

    #[test]
    fn test_message_with_emoji() {
        let raw = "@user-id=1;display-name=Test :t PRIVMSG #ch :hello 🎉 world";
        let event = parse_one(raw);
        assert_chat_message(&event, "1", "Test", TwitchRole::empty(), "hello 🎉 world");
    }

    #[test]
    fn test_message_with_semicolons() {
        let raw = "@user-id=1;display-name=Test :t PRIVMSG #ch :a;b;c;d";
        let event = parse_one(raw);
        assert_chat_message(&event, "1", "Test", TwitchRole::empty(), "a;b;c;d");
    }

    #[test]
    fn test_message_without_tags() {
        let raw = ":username!username@username.tmi.twitch.tv PRIVMSG #channel :hello";
        let event = parse_one(raw);
        assert_chat_message(&event, "0", "anon", TwitchRole::empty(), "hello");
    }

    #[test]
    fn test_channel_extraction() {
        let raw = "@user-id=1;display-name=Test :t PRIVMSG #mychannel :hello";
        let event = parse_one(raw);
        match event {
            TwitchEvent::ChatMessage { channel, .. } => {
                assert_eq!(channel, Some("mychannel".to_string()));
            }
            _ => panic!("Expected ChatMessage"),
        }
    }
}
