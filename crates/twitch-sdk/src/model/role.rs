#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct TwitchRole(u8);

impl TwitchRole {
    const BIT_SUBSCRIBER: u8 = 1 << 0;
    const BIT_VIP: u8 = 1 << 1;
    const BIT_MODERATOR: u8 = 1 << 2;
    const BIT_BROADCASTER: u8 = 1 << 3;

    pub const SUBSCRIBER: TwitchRole = TwitchRole(Self::BIT_SUBSCRIBER);
    pub const VIP: TwitchRole = TwitchRole(Self::BIT_VIP);
    pub const MODERATOR: TwitchRole = TwitchRole(Self::BIT_MODERATOR);
    pub const BROADCASTER: TwitchRole = TwitchRole(Self::BIT_BROADCASTER);

    #[must_use]
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn add(&mut self, other: TwitchRole) {
        self.0 |= other.0;
    }

    #[must_use]
    pub fn highest(&self) -> TwitchRole {
        const PRIORITY: [u8; 4] = [
            TwitchRole::BIT_BROADCASTER,
            TwitchRole::BIT_MODERATOR,
            TwitchRole::BIT_VIP,
            TwitchRole::BIT_SUBSCRIBER,
        ];
        for bit in PRIORITY {
            if self.0 & bit != 0 {
                return TwitchRole(bit);
            }
        }
        TwitchRole(0)
    }
}
