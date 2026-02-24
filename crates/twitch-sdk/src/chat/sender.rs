use crate::chat::traits::Auth;

#[non_exhaustive]
pub struct TwitchSender<T> {
    auth: T,
}

impl<T: Auth> TwitchSender<T> {
    pub fn new(auth: T) -> Self {
        Self { auth }
    }
}
