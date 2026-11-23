//! Challenge kinds (parent spec §5.3): how a kind turns an answer into the
//! two results the rules compare.
//!
//! Each kind is a [`ChallengeKind`] in the [`KindRegistry`]; adding one is a
//! new module and one line in [`KindRegistry::standard`].

mod query;

use quarry_engine::result::QueryResult;

use crate::{check::CheckError, run::Runner, sandbox::Template, track::Challenge};

/// The user's result and the expected one, for the rules to compare.
#[derive(Debug, Clone, PartialEq)]
pub struct Answers {
    /// What the reference solution gives.
    pub expected: QueryResult,
    /// What the user's SQL gave.
    pub got: QueryResult,
}

/// One kind of challenge.
pub trait ChallengeKind: Send + Sync {
    /// Its id, as `kind` in `challenges.toml`, such as "query".
    fn id(&self) -> &'static str;

    /// What a right answer to `challenge` gives, run by `runner` on a fresh
    /// copy of `template`.
    ///
    /// # Errors
    /// Returns [`CheckError::Content`] when the solution gives no result, or
    /// [`CheckError::Sandbox`] when no copy can be made.
    fn expected(
        &self,
        challenge: &Challenge,
        template: &Template,
        runner: &Runner,
    ) -> Result<QueryResult, CheckError>;

    /// Runs `sql` and `challenge`'s solution with `runner`, each on a
    /// fresh copy of `template`.
    ///
    /// # Errors
    /// Returns a [`CheckError`] when either cannot give a result.
    fn answers(
        &self,
        challenge: &Challenge,
        sql: &str,
        template: &Template,
        runner: &Runner,
    ) -> Result<Answers, CheckError>;
}

/// The kinds a challenge can be.
pub struct KindRegistry {
    kinds: Vec<Box<dyn ChallengeKind>>,
}

impl KindRegistry {
    /// Query; Mutation arrives in Phase 2D.
    pub fn standard() -> Self {
        Self {
            kinds: vec![Box::new(query::Query)],
        }
    }

    /// The kind with id `id`.
    pub fn get(&self, id: &str) -> Option<&dyn ChallengeKind> {
        self.kinds
            .iter()
            .find(|kind| kind.id() == id)
            .map(AsRef::as_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::{ChallengeKind, KindRegistry};

    #[test]
    fn query_is_the_only_kind_so_far() {
        let kinds = KindRegistry::standard();
        assert_eq!(kinds.get("query").map(ChallengeKind::id), Some("query"));
        assert!(kinds.get("mutation").is_none());
    }
}
