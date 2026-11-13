//! The Messages view: one line per statement of a run.

use quarry_engine::error::EngineError;

use super::{statements::RunOutput, status};

/// One statement's line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageLine {
    /// The statement's label, as in "2 · INSERT INTO crew…".
    pub label: String,
    /// What happened: rows, rows affected, "Done", the error, or "Cancelled".
    pub text: String,
    /// Whether the line reports an error (shown in the danger colour).
    pub is_error: bool,
}

/// A line for every statement that started, in order.
pub fn lines(output: &RunOutput) -> Vec<MessageLine> {
    output
        .statements()
        .iter()
        .map(|statement| {
            let (text, is_error) = match statement.outcome() {
                None => ("Running…".to_owned(), false),
                Some(Ok(summary)) => (status::outcome_text(statement, summary), false),
                Some(Err(EngineError::Cancelled)) => ("Cancelled".to_owned(), false),
                Some(Err(error)) => (error.to_string(), true),
            };
            MessageLine {
                label: statement.label().to_owned(),
                text,
                is_error,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use quarry_engine::{error::EngineError, result::Summary, session::RunEvent};

    use super::{MessageLine, lines};
    use crate::model::statements::RunOutput;

    fn line(label: &str, text: &str, is_error: bool) -> MessageLine {
        MessageLine {
            label: label.to_owned(),
            text: text.to_owned(),
            is_error,
        }
    }

    #[test]
    fn every_started_statement_gets_a_line() {
        let script = "INSERT INTO t VALUES (1);SELECT 1;SELECT x;SELECT 2";
        let mut output = RunOutput::submitted(script);
        let events = vec![
            RunEvent::Started {
                index: 0,
                range: 0..24,
            },
            RunEvent::Finished(Summary {
                rows: 0,
                rows_affected: Some(2),
                elapsed: Duration::from_millis(1),
            }),
            RunEvent::Started {
                index: 1,
                range: 25..33,
            },
            RunEvent::Failed(EngineError::Sql("no such column: x".to_owned())),
        ];
        for event in events {
            output.apply(event, Instant::now());
        }
        assert_eq!(
            lines(&output),
            [
                line(
                    "1 · INSERT INTO t VALUES (1)",
                    "2 rows affected · 1.0 ms",
                    false
                ),
                line("2 · SELECT 1", "no such column: x", true),
            ]
        );
    }

    #[test]
    fn running_and_cancelled_statements_are_not_errors() {
        let mut output = RunOutput::submitted("SELECT 1;SELECT 2");
        output.apply(
            RunEvent::Started {
                index: 0,
                range: 0..8,
            },
            Instant::now(),
        );
        assert_eq!(lines(&output), [line("1 · SELECT 1", "Running…", false)]);
        output.apply(RunEvent::Failed(EngineError::Cancelled), Instant::now());
        assert_eq!(lines(&output), [line("1 · SELECT 1", "Cancelled", false)]);
    }
}
