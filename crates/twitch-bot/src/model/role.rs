#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Role(u8);

impl Role {
    const BIT_SUBSCRIBER: u8 = 1 << 0;
    const BIT_VIP: u8 = 1 << 1;
    const BIT_MODERATOR: u8 = 1 << 2;
    const BIT_BROADCASTER: u8 = 1 << 3;

    pub const PLEB: Role = Role(0);
    pub const SUBSCRIBER: Role = Role(Self::BIT_SUBSCRIBER);
    pub const VIP: Role = Role(Self::BIT_VIP | Self::BIT_SUBSCRIBER);
    pub const MODERATOR: Role = Role(Self::BIT_MODERATOR | Self::BIT_VIP | Self::BIT_SUBSCRIBER);
    pub const BROADCASTER: Role =
        Role(Self::BIT_BROADCASTER | Self::BIT_MODERATOR | Self::BIT_VIP | Self::BIT_SUBSCRIBER);

    #[must_use]
    pub fn new() -> Self {
        Self::PLEB
    }

    #[must_use]
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn add(&mut self, other: Role) {
        self.0 |= other.0;
    }

    pub fn contains(&self, other: Role) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn is_broadcaster(&self) -> bool {
        self.contains(Self::BROADCASTER)
    }

    pub fn is_moderator(&self) -> bool {
        self.contains(Self::MODERATOR)
    }

    pub fn is_vip(&self) -> bool {
        self.contains(Self::VIP)
    }

    pub fn is_subscriber(&self) -> bool {
        self.contains(Self::SUBSCRIBER)
    }
}
