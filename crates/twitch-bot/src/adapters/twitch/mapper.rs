use twitch_sdk::event::{self as twitch, Badge};

use crate::model::{ChatMessage, ChatTarget, Event, RewardRedemption, Role, User};

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
            author_role: message
                .badges
                .iter()
                .map(Role::from)
                .max()
                .unwrap_or_default(),
            author: message.chatter.into(),
            target: ChatTarget {
                broadcaster_id: message.broadcaster.id,
            },
            text: message.text,
        }
    }
}

impl From<twitch::RewardRedemption> for RewardRedemption {
    fn from(redemption: twitch::RewardRedemption) -> Self {
        Self {
            user: redemption.user.into(),
            reward_id: redemption.reward.id,
            reward_title: redemption.reward.title,
            cost: redemption.reward.cost,
            user_input: redemption.user_input,
        }
    }
}

impl From<twitch::User> for User {
    fn from(user: twitch::User) -> Self {
        Self {
            id: user.id,
            display_name: user.display_name,
        }
    }
}

impl From<&Badge> for Role {
    fn from(badge: &Badge) -> Self {
        match badge {
            Badge::Broadcaster => Self::Broadcaster,
            Badge::Moderator => Self::Moderator,
            Badge::Vip => Self::Vip,
            Badge::Subscriber | Badge::Founder => Self::Subscriber,
            Badge::Unknown(_) => Self::Viewer,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role(badges: &[Badge]) -> Role {
        badges.iter().map(Role::from).max().unwrap_or_default()
    }

    #[test]
    fn highest_badge_becomes_role() {
        assert_eq!(
            role(&[Badge::Subscriber, Badge::Moderator]),
            Role::Moderator
        );
        assert_eq!(role(&[Badge::Broadcaster]), Role::Broadcaster);
        assert_eq!(role(&[Badge::Founder]), Role::Subscriber);
        assert_eq!(role(&[Badge::Unknown("premium".into())]), Role::Viewer);
        assert_eq!(role(&[]), Role::Viewer);
    }
}
