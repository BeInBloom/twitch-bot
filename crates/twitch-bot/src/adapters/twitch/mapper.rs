use std::time::SystemTime;

use twitch_sdk::event::{self as twitch, Badge};

use crate::model::{ChatMessage, ChatTarget, Event, Platform, RewardRedemption, Role, User};

impl From<twitch::Event> for Event {
    fn from(event: twitch::Event) -> Self {
        match event {
            twitch::Event::ChatMessage(message) => Self::ChatMessage(Box::new((*message).into())),
            twitch::Event::RewardRedemption(redemption) => {
                Self::RewardRedemption(Box::new((*redemption).into()))
            }
        }
    }
}

impl From<twitch::ChatMessage> for ChatMessage {
    fn from(message: twitch::ChatMessage) -> Self {
        Self {
            author: user(message.chatter, Role::from(message.badges.as_slice())),
            target: ChatTarget::new(message.broadcaster.id, message.broadcaster.login),
            text: message.text,
            received_at: SystemTime::now(),
        }
    }
}

impl From<twitch::RewardRedemption> for RewardRedemption {
    fn from(redemption: twitch::RewardRedemption) -> Self {
        Self {
            user: user(redemption.user, Role::PLEB),
            reward_id: redemption.reward.id,
            reward_title: redemption.reward.title,
            cost: redemption.reward.cost,
            user_input: redemption.user_input,
            received_at: SystemTime::now(),
        }
    }
}

impl From<&[Badge]> for Role {
    fn from(badges: &[Badge]) -> Self {
        let has = |badge: Badge| badges.contains(&badge);

        if has(Badge::Broadcaster) {
            Self::BROADCASTER
        } else if has(Badge::Moderator) {
            Self::MODERATOR
        } else if has(Badge::Vip) {
            Self::VIP
        } else if has(Badge::Subscriber) || has(Badge::Founder) {
            Self::SUBSCRIBER
        } else {
            Self::PLEB
        }
    }
}

fn user(user: twitch::User, role: Role) -> User {
    User {
        id: user.id,
        display_name: user.display_name,
        platform: Platform::Twitch,
        role,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highest_badge_becomes_role() {
        assert_eq!(
            Role::from([Badge::Subscriber, Badge::Moderator].as_slice()),
            Role::MODERATOR
        );
        assert_eq!(
            Role::from([Badge::Broadcaster].as_slice()),
            Role::BROADCASTER
        );
        assert_eq!(Role::from([Badge::Founder].as_slice()), Role::SUBSCRIBER);
        assert_eq!(
            Role::from([Badge::Unknown("premium".into())].as_slice()),
            Role::PLEB
        );
    }
}
