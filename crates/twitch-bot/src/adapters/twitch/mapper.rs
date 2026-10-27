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
    Event::ChatMessage(ChatMessage {
        author: map_user(user),
        target: ChatTarget::new(target.broadcaster_id, target.channel_login),
        text,
        received_at: SystemTime::now(),
    })
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
}
