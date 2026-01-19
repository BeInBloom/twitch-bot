use crate::domain::models::{Event, Platform, Role, User};

pub trait MessageParser: Send + Sync + Clone {
    fn parse(&self, raw: &str) -> Vec<Event>;
}

#[derive(Clone, Copy, Default)]
pub struct TwitchIrcParser;

impl TwitchIrcParser {
    pub fn new() -> Self {
        Self
    }
}

impl MessageParser for TwitchIrcParser {
    fn parse(&self, raw: &str) -> Vec<Event> {
        raw.split('\n')
            .map(|line| line.strip_suffix('\r').unwrap_or(line))
            .filter(|line| !line.is_empty())
            .filter_map(parse_line)
            .collect()
    }
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

fn parse_line(line: &str) -> Option<Event> {
    let msg = parse_irc_structure(line)?;

    match msg.command {
        "PRIVMSG" => parse_privmsg(msg.tags, msg.params),
        _ => None,
    }
}

fn parse_privmsg(tags: &str, params: &str) -> Option<Event> {
    let (_, text) = params.split_once(" :")?;

    let meta = parse_tags(tags);

    Some(Event::ChatMessage {
        user: User {
            id: meta.user_id.to_string(),
            display_name: meta.display_name.to_string(),
            platform: Platform::Twitch,
            role: meta.role,
        },
        text: text.to_string(),
    })
}

struct UserMeta<'a> {
    user_id: &'a str,
    display_name: &'a str,
    role: Role,
}

fn parse_badges(badges: &str) -> Role {
    let mut role = Role::new();

    for badge in badges.split(',') {
        match badge {
            _ if badge.starts_with("broadcaster/") => role.add(Role::BROADCASTER),
            _ if badge.starts_with("vip/") => role.add(Role::VIP),
            _ if badge.starts_with("subscriber/") => role.add(Role::SUBSCRIBER),
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
            role: Role::new(),
        };
    }

    let mut user_id = "0";
    let mut display_name: Option<&str> = None;
    let mut login: Option<&str> = None;
    let mut role = Role::new();

    for pair in tags.split(';') {
        let Some((key, val)) = pair.split_once('=') else {
            continue;
        };

        match key {
            "user-id" => user_id = val,
            "display-name" if !val.is_empty() => display_name = Some(val),
            "login" => login = Some(val),
            "mod" if val == "1" => role.add(Role::MODERATOR),
            "subscriber" if val == "1" => role.add(Role::SUBSCRIBER),
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

    fn parse_one(raw: &str) -> Event {
        let parser = TwitchIrcParser::new();
        let events = parser.parse(raw);
        assert_eq!(
            events.len(),
            1,
            "Expected exactly 1 event, got {}",
            events.len()
        );
        events.into_iter().next().unwrap()
    }

    fn assert_chat_message(
        event: &Event,
        expected_id: &str,
        expected_name: &str,
        expected_role: Role,
        expected_text: &str,
    ) {
        match event {
            Event::ChatMessage { user, text } => {
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

    // ========== Basic parsing ==========

    fn role(flag: u8) -> Role {
        let mut r = Role::new();
        r.add(flag);
        r
    }

    #[test]
    fn test_parse_privmsg() {
        let raw = "@badge-info=;badges=broadcaster/1;display-name=TestUser;mod=0;user-id=12345 :testuser!testuser@testuser.tmi.twitch.tv PRIVMSG #channel :Hello world!";
        let event = parse_one(raw);
        assert_chat_message(
            &event,
            "12345",
            "TestUser",
            role(Role::BROADCASTER),
            "Hello world!",
        );
    }

    #[test]
    fn test_parse_multiline() {
        let parser = TwitchIrcParser::new();
        let raw = "@user-id=1;display-name=User1 :u1 PRIVMSG #ch :msg1\r\n@user-id=2;display-name=User2 :u2 PRIVMSG #ch :msg2";
        let events = parser.parse(raw);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_parse_multiline_unix_newlines() {
        let parser = TwitchIrcParser::new();
        let raw = "@user-id=1;display-name=User1 :u1 PRIVMSG #ch :msg1\n@user-id=2;display-name=User2 :u2 PRIVMSG #ch :msg2";
        let events = parser.parse(raw);
        assert_eq!(events.len(), 2);
    }

    // ========== Role detection ==========

    #[test]
    fn test_parse_broadcaster() {
        let raw = "@badges=broadcaster/1;display-name=Streamer;user-id=1 :s PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(&event, "1", "Streamer", role(Role::BROADCASTER), "hi");
    }

    #[test]
    fn test_parse_mod() {
        let raw = "@mod=1;display-name=ModUser;user-id=2 :m PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(&event, "2", "ModUser", role(Role::MODERATOR), "hi");
    }

    #[test]
    fn test_parse_vip() {
        let raw = "@badges=vip/1;display-name=VipUser;user-id=3 :v PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(&event, "3", "VipUser", role(Role::VIP), "hi");
    }

    #[test]
    fn test_parse_subscriber() {
        let raw = "@subscriber=1;display-name=SubUser;user-id=4 :s PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(&event, "4", "SubUser", role(Role::SUBSCRIBER), "hi");
    }

    #[test]
    fn test_parse_subscriber_badge() {
        let raw = "@badges=subscriber/12;display-name=SubUser;user-id=5 :s PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(&event, "5", "SubUser", role(Role::SUBSCRIBER), "hi");
    }

    #[test]
    fn test_parse_pleb() {
        let raw = "@display-name=PlebUser;user-id=6 :p PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(&event, "6", "PlebUser", Role::new(), "hi");
    }

    #[test]
    fn test_role_priority_broadcaster_over_mod() {
        // Broadcaster badge + mod=1 should be Admin (broadcaster wins)
        let raw = "@badges=broadcaster/1;mod=1;display-name=Test;user-id=1 :t PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(
            &event,
            "1",
            "Test",
            role(Role::BROADCASTER | Role::MODERATOR),
            "hi",
        );
    }

    #[test]
    fn test_role_priority_mod_over_vip() {
        let raw = "@badges=vip/1;mod=1;display-name=Test;user-id=1 :t PRIVMSG #ch :hi";
        let event = parse_one(raw);
        assert_chat_message(&event, "1", "Test", role(Role::MODERATOR | Role::VIP), "hi");
    }

    // ========== Display name fallback ==========

    #[test]
    fn test_empty_display_name_fallback_to_login() {
        let raw = "@display-name=;login=mylogin;user-id=123 :user PRIVMSG #ch :test";
        let event = parse_one(raw);
        assert_chat_message(&event, "123", "mylogin", Role::new(), "test");
    }

    #[test]
    fn test_missing_display_name_fallback_to_login() {
        let raw = "@login=fallbacklogin;user-id=456 :user PRIVMSG #ch :test";
        let event = parse_one(raw);
        assert_chat_message(&event, "456", "fallbacklogin", Role::new(), "test");
    }

    #[test]
    fn test_no_name_tags_fallback_to_anon() {
        let raw = "@user-id=789 :user PRIVMSG #ch :test";
        let event = parse_one(raw);
        assert_chat_message(&event, "789", "anon", Role::new(), "test");
    }

    // ========== Edge cases ==========

    #[test]
    fn test_empty_input() {
        let parser = TwitchIrcParser::new();
        let events = parser.parse("");
        assert!(events.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let parser = TwitchIrcParser::new();
        let events = parser.parse("   \n\r\n   ");
        assert!(events.is_empty());
    }

    #[test]
    fn test_non_privmsg_ignored() {
        let parser = TwitchIrcParser::new();

        // PING should be ignored (handled separately in fetcher)
        let events = parser.parse("PING :tmi.twitch.tv");
        assert!(events.is_empty());

        // JOIN should be ignored
        let events = parser.parse(":user!user@user.tmi.twitch.tv JOIN #channel");
        assert!(events.is_empty());

        // NOTICE should be ignored
        let events = parser.parse(
            "@msg-id=slow_off :tmi.twitch.tv NOTICE #channel :This room is no longer in slow mode.",
        );
        assert!(events.is_empty());
    }

    #[test]
    fn test_malformed_no_command() {
        let parser = TwitchIrcParser::new();
        let events = parser.parse("@tags-only-no-rest");
        assert!(events.is_empty());
    }

    #[test]
    fn test_malformed_no_message() {
        let parser = TwitchIrcParser::new();
        let events = parser.parse("@user-id=1 :user PRIVMSG #channel");
        assert!(events.is_empty());
    }

    // ========== Special characters in messages ==========

    #[test]
    fn test_message_with_colon() {
        let raw = "@user-id=1;display-name=Test :t PRIVMSG #ch :hello: world: test";
        let event = parse_one(raw);
        assert_chat_message(&event, "1", "Test", Role::new(), "hello: world: test");
    }

    #[test]
    fn test_message_with_url() {
        let raw = "@user-id=1;display-name=Test :t PRIVMSG #ch :check https://example.com/page";
        let event = parse_one(raw);
        assert_chat_message(
            &event,
            "1",
            "Test",
            Role::new(),
            "check https://example.com/page",
        );
    }

    #[test]
    fn test_message_with_emoji() {
        let raw = "@user-id=1;display-name=Test :t PRIVMSG #ch :hello 🎉 world";
        let event = parse_one(raw);
        assert_chat_message(&event, "1", "Test", Role::new(), "hello 🎉 world");
    }

    #[test]
    fn test_message_with_semicolons() {
        let raw = "@user-id=1;display-name=Test :t PRIVMSG #ch :a;b;c;d";
        let event = parse_one(raw);
        assert_chat_message(&event, "1", "Test", Role::new(), "a;b;c;d");
    }

    // ========== No tags ==========

    #[test]
    fn test_message_without_tags() {
        let raw = ":username!username@username.tmi.twitch.tv PRIVMSG #channel :hello";
        let event = parse_one(raw);
        assert_chat_message(&event, "0", "anon", Role::new(), "hello");
    }
}
