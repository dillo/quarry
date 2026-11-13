//! Takes everything waiting on a channel without blocking or awaiting.
//!
//! The UI polls the session's channels every `POLL` (33 ms) instead of
//! awaiting them: a waker fired from the session thread would wake GPUI's
//! executor from outside, and polling also batches many row events into one
//! redraw.

use async_channel::{Receiver, TryRecvError};
use quarry_engine::error::EngineError;

/// What one poll found.
#[derive(Debug, PartialEq)]
pub struct Drained<T> {
    /// Every message that was waiting, in order.
    pub items: Vec<T>,
    /// Whether the sender is gone, so nothing more will arrive.
    pub closed: bool,
}

/// Takes every waiting message from `receiver`.
pub fn drain<T>(receiver: &Receiver<T>) -> Drained<T> {
    let mut items = Vec::new();
    loop {
        match receiver.try_recv() {
            Ok(item) => items.push(item),
            Err(TryRecvError::Empty) => {
                return Drained {
                    items,
                    closed: false,
                };
            }
            Err(TryRecvError::Closed) => {
                return Drained {
                    items,
                    closed: true,
                };
            }
        }
    }
}

/// A one-off reply (a schema or a plan), once it has come: the reply
/// itself, or `WorkerGone` when the channel closed without one.
pub fn reply<T>(drained: Drained<Result<T, EngineError>>) -> Option<Result<T, EngineError>> {
    let Drained { mut items, closed } = drained;
    items
        .pop()
        .or_else(|| closed.then_some(Err(EngineError::WorkerGone)))
}

#[cfg(test)]
mod tests {
    use quarry_engine::error::EngineError;

    use super::{Drained, drain, reply};

    #[test]
    fn takes_everything_waiting_and_stays_open() {
        let (sender, receiver) = async_channel::unbounded();
        sender.try_send(1).unwrap();
        sender.try_send(2).unwrap();
        assert_eq!(
            drain(&receiver),
            Drained {
                items: vec![1, 2],
                closed: false
            }
        );
        assert_eq!(
            drain(&receiver),
            Drained {
                items: vec![],
                closed: false
            }
        );
    }

    #[test]
    fn reports_a_closed_channel_after_its_last_items() {
        let (sender, receiver) = async_channel::unbounded();
        sender.try_send(7).unwrap();
        drop(sender);
        assert_eq!(
            drain(&receiver),
            Drained {
                items: vec![7],
                closed: true
            }
        );
    }

    #[test]
    fn a_reply_is_its_last_item_or_a_lost_worker() {
        let waiting: Drained<Result<u8, EngineError>> = Drained {
            items: vec![],
            closed: false,
        };
        assert_eq!(reply(waiting), None);
        let came = Drained {
            items: vec![Ok(1), Ok(2)],
            closed: true,
        };
        assert_eq!(reply(came), Some(Ok(2)));
        let gone: Drained<Result<u8, EngineError>> = Drained {
            items: vec![],
            closed: true,
        };
        assert_eq!(reply(gone), Some(Err(EngineError::WorkerGone)));
    }
}
