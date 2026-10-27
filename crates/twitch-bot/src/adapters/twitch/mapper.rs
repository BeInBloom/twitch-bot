use std::time::SystemTime;

use twitch_sdk::{TwitchChatTarget, TwitchEvent, TwitchRole, TwitchUser};

use crate::model::{
    ChatMessage, ChatTarget, Event, Platform, RewardRedemption, Role, UnsupportedEvent, User,
};

pub(crate) fn map_event(event: TwitchEvent) -> Event {
    match event {
        TwitchEvent::ChatMessage { user, target, text } => map_chat_message(user, target, text),
        TwitchEvent::RewardRedemption {
            user,
            reward_id,
            reward_title,
            cost,
            user_input,
        } => Event::RewardRedemption(RewardRedemption {
            user: map_user(user),
            reward_id,
            reward_title,
            cost,
            user_input,
            received_at: SystemTime::now(),
        }),
        _ => Event::Unsupported(UnsupportedEvent {
            description: "unsupported twitch event".to_string(),
            received_at: SystemTime::now(),
        }),
    }
}

fn map_chat_message(user: TwitchUser, target: TwitchChatTarget, text: String) -> Event {
    let user = map_user(user);
    let Some(target) = map_chat_target(target) else {
        return Event::Unsupported(UnsupportedEvent {
            description: "chat message without a complete chat target".to_string(),
            received_at: SystemTime::now(),
        });
    };

    Event::ChatMessage(ChatMessage {
        author: user,
        target,
        text,
        received_at: SystemTime::now(),
    })
}

fn map_chat_target(target: TwitchChatTarget) -> Option<ChatTarget> {
    let broadcaster_id = target.broadcaster_id?;
    let channel_login = target.channel_login?;
    Some(ChatTarget::new(broadcaster_id, channel_login))
}

fn map_user(user: TwitchUser) -> User {
    User {
        id: user.id,
        display_name: user.display_name,
        platform: Platform::Twitch,
        role: map_role(user.role),
    }
}

fn map_role(role: TwitchRole) -> Role {
    match role.highest() {
        TwitchRole::BROADCASTER => Role::BROADCASTER,
        TwitchRole::MODERATOR => Role::MODERATOR,
        TwitchRole::VIP => Role::VIP,
        TwitchRole::SUBSCRIBER => Role::SUBSCRIBER,
        _ => Role::PLEB,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_highest_twitch_role() {
        let mut moderator_and_subscriber = TwitchRole::empty();
        moderator_and_subscriber.add(TwitchRole::SUBSCRIBER);
        moderator_and_subscriber.add(TwitchRole::MODERATOR);

        assert_eq!(map_role(moderator_and_subscriber), Role::MODERATOR);
        assert_eq!(map_role(TwitchRole::BROADCASTER), Role::BROADCASTER);
        assert_eq!(map_role(TwitchRole::empty()), Role::PLEB);
    }

    #[test]
    fn chat_target_requires_broadcaster_id_and_login() {
        let mut target = TwitchChatTarget::default();
        target.channel_login = Some("channel".into());
        assert!(map_chat_target(target.clone()).is_none());

        target.broadcaster_id = Some("1".into());
        assert_eq!(
            map_chat_target(target),
            Some(ChatTarget::new("1", "channel"))
        );
    }
}
