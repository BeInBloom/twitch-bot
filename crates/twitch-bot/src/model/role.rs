use std::ops::{BitOr, BitOrAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Role(u8);

impl Role {
    const BIT_SUBSCRIBER: u8 = 1 << 0;
    const BIT_VIP: u8 = 1 << 1;
    const BIT_MODERATOR: u8 = 1 << 2;
    const BIT_BROADCASTER: u8 = 1 << 3;

    pub const PLEB: Self = Self(0);
    pub const SUBSCRIBER: Self = Self(Self::BIT_SUBSCRIBER);
    pub const VIP: Self = Self(Self::BIT_VIP | Self::BIT_SUBSCRIBER);
    pub const MODERATOR: Self = Self(Self::BIT_MODERATOR | Self::BIT_VIP | Self::BIT_SUBSCRIBER);
    pub const BROADCASTER: Self =
        Self(Self::BIT_BROADCASTER | Self::BIT_MODERATOR | Self::BIT_VIP | Self::BIT_SUBSCRIBER);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn is_broadcaster(self) -> bool {
        self.contains(Self::BROADCASTER)
    }

    pub const fn is_moderator(self) -> bool {
        self.contains(Self::MODERATOR)
    }

    pub const fn is_vip(self) -> bool {
        self.contains(Self::VIP)
    }

    pub const fn is_subscriber(self) -> bool {
        self.contains(Self::SUBSCRIBER)
    }
}

impl BitOr for Role {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl BitOrAssign for Role {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}
