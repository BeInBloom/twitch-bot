#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Role {
    #[default]
    Viewer,
    Subscriber,
    Vip,
    Moderator,
    Broadcaster,
}
