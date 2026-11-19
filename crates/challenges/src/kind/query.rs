//! Query challenges: the user's result is compared with the reference
//! solution's, each run on its own fresh copy.

use quarry_engine::sqlite::SqliteDriver;

use super::{Answers, ChallengeKind};
use crate::{
    check::CheckError,
    run::{self, Limits, RunError},
    sandbox::Template,
    track::Challenge,
};

/// A challenge answered by a query.
pub struct Query;

impl ChallengeKind for Query {
    fn id(&self) -> &'static str {
        "query"
    }

    fn answers(
        &self,
        challenge: &Challenge,
        sql: &str,
        template: &Template,
        limits: Limits,
    ) -> Result<Answers, CheckError> {
        let expected = run::run(&mut copy(template)?, &challenge.solution, limits)
            .map_err(|error| CheckError::Content(describe(&error)))?;
        let got = run::run(&mut copy(template)?, sql, limits).map_err(CheckError::from)?;
        Ok(Answers { expected, got })
    }
}

/// A fresh copy of `template`.
fn copy(template: &Template) -> Result<SqliteDriver, CheckError> {
    template.fresh().map_err(CheckError::Sandbox)
}

/// Why the reference solution gave no result, for the content's authors.
fn describe(error: &RunError) -> String {
    match error {
        RunError::Sql(error) => error.to_string(),
        RunError::TimedOut => "the solution ran out of time".to_owned(),
        RunError::TooManyRows => "the solution returned too many rows".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use quarry_engine::{error::EngineError, value::Value};

    use super::{Query, describe};
    use crate::{
        check::CheckError,
        kind::ChallengeKind,
        run::{LIMITS, Limits, RunError},
        sandbox::Template,
        track::Challenge,
    };

    fn challenge(solution: &str) -> Challenge {
        Challenge {
            id: "c".to_owned(),
            title: "C".to_owned(),
            prompt: "P".to_owned(),
            kind: "query".to_owned(),
            ordered: false,
            concepts: Vec::new(),
            solution: solution.to_owned(),
            hints: Vec::new(),
            skeleton: String::new(),
        }
    }

    fn template() -> Result<Template, EngineError> {
        Template::new(
            "CREATE TABLE t (n INTEGER);",
            "INSERT INTO t VALUES (1), (2);",
        )
    }

    #[test]
    fn each_side_runs_on_its_own_fresh_copy() {
        let sql = "DELETE FROM t; SELECT count(*) AS n FROM t";
        let answers = Query
            .answers(
                &challenge("SELECT count(*) AS n FROM t"),
                sql,
                &template().unwrap(),
                LIMITS,
            )
            .unwrap();
        assert_eq!(answers.expected.rows, [[Value::Int(2)]]);
        assert_eq!(answers.got.rows, [[Value::Int(0)]]);
    }

    #[test]
    fn the_users_errors_keep_their_kind() {
        let template = template().unwrap();
        let solution = challenge("SELECT n FROM t");
        let failed = Query.answers(&solution, "SELECT x FROM t", &template, LIMITS);
        assert_eq!(failed, Err(CheckError::Sql("no such column: x".to_owned())));
        let tight = Limits {
            rows: 1,
            time: LIMITS.time,
        };
        let too_many = Query.answers(&challenge("SELECT 1"), "SELECT n FROM t", &template, tight);
        assert_eq!(too_many, Err(CheckError::TooManyRows));
    }

    #[test]
    fn a_failing_solution_is_a_content_error() {
        let broken = Query.answers(
            &challenge("SELECT nope FROM t"),
            "SELECT 1",
            &template().unwrap(),
            LIMITS,
        );
        assert_eq!(
            broken,
            Err(CheckError::Content("no such column: nope".to_owned()))
        );
    }

    #[test]
    fn content_errors_say_what_stopped_the_solution() {
        assert_eq!(
            describe(&RunError::TimedOut),
            "the solution ran out of time"
        );
        assert_eq!(
            describe(&RunError::TooManyRows),
            "the solution returned too many rows"
        );
    }
}
