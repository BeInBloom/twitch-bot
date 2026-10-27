pub mod message_sink;
pub mod now_playing;
#[cfg(test)]
pub(crate) mod testing;

pub use message_sink::MessageSink;
pub use now_playing::{MusicSkipProvider, NowPlayingProvider};
