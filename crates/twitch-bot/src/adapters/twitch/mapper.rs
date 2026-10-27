use std::time::SystemTime;

use twitch_sdk::event::{self as twitch, Badge};

use crate::model::{ChatMessage, ChatTarget, Event, Platform, RewardRedemption, Role, User};

pub(crate) fn map_event(event: twitch::Event) -> Event {
    match event {
        twitch::Event::ChatMessage(message) => Event::ChatMessage(map_chat_message(message)),
        twitch::Event::RewardRedemption(redemption) => {
            Event::RewardRedemption(map_reward_redemption(redemption))
        }
    }
}

fn map_chat_message(message: twitch::ChatMessage) -> ChatMessage {
    ChatMessage {
        author: map_user(message.chatter, map_role(&message.badges)),
        target: ChatTarget::new(message.broadcaster.id, message.broadcaster.login),
        text: message.text,
        received_at: SystemTime::now(),
    }
}

fn map_reward_redemption(redemption: twitch::RewardRedemption) -> RewardRedemption {
    RewardRedemption {
        user: map_user(redemption.user, Role::PLEB),
        reward_id: redemption.reward.id,
        reward_title: redemption.reward.title,
        cost: redemption.reward.cost,
        user_input: redemption.user_input,
        received_at: SystemTime::now(),
    }
}

fn map_user(user: twitch::User, role: Role) -> User {
    User {
        id: user.id,
        display_name: user.display_name,
        platform: Platform::Twitch,
        role,
    }
}

fn map_role(badges: &[Badge]) -> Role {
    let has = |badge: Badge| badges.contains(&badge);

    if has(Badge::Broadcaster) {
        Role::BROADCASTER
    } else if has(Badge::Moderator) {
        Role::MODERATOR
    } else if has(Badge::Vip) {
        Role::VIP
    } else if has(Badge::Subscriber) || has(Badge::Founder) {
        Role::SUBSCRIBER
    } else {
        Role::PLEB
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_highest_badge_to_role() {
        assert_eq!(
            map_role(&[Badge::Subscriber, Badge::Moderator]),
            Role::MODERATOR
        );
        assert_eq!(map_role(&[Badge::Broadcaster]), Role::BROADCASTER);
        assert_eq!(map_role(&[Badge::Founder]), Role::SUBSCRIBER);
        assert_eq!(map_role(&[Badge::Unknown("premium".into())]), Role::PLEB);
    }
}
