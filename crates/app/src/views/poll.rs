//! Waits for a one-off reply from the session, such as a schema or a plan.
//!
//! The reply is polled once per [`POLL`] rather than awaited (see
//! [`crate::model::drain`]).

use async_channel::Receiver;
use gpui_kit::BackgroundExecutor;
use quarry_engine::error::EngineError;

use super::workspace::POLL;
use crate::model::drain;

/// The answer on `reply`, polled every [`POLL`] on `executor`'s clock.
///
/// # Errors
/// Returns the session's error, or [`EngineError::WorkerGone`] when the
/// channel closes without an answer.
pub async fn poll_reply<T>(
    reply: Receiver<Result<T, EngineError>>,
    executor: BackgroundExecutor,
) -> Result<T, EngineError> {
    loop {
        executor.timer(POLL).await;
        if let Some(answer) = drain::reply(drain::drain(&reply)) {
            return answer;
        }
    }
}
