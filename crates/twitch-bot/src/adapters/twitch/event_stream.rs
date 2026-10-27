use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};
use twitch_sdk::eventsub::{EventSub, EventSubHandle};

use crate::model::Event;

pub fn start_events(
    eventsub: EventSub,
) -> (
    impl Stream<Item = Event> + Send + Unpin + 'static,
    EventSubHandle,
) {
    let (events, handle) = eventsub.spawn();
    (ReceiverStream::new(events).map(Event::from), handle)
}
