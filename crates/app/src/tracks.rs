//! The challenge tracks and their sandboxes.
//!
//! Each track's template database is built the first time one of its
//! challenges is opened, then kept; every challenge tab runs on a fresh
//! copy of it.

use std::{collections::BTreeMap, sync::Arc};

use quarry_challenges::{
    catalog::{Catalog, ChallengeId},
    sandbox::Template,
    track::{Challenge, Track},
};
use quarry_engine::{error::EngineError, session::Session};

use crate::model::brief::Expected;

/// The tracks embedded in Quarry, the templates built so far, and what
/// is known of each challenge's expected result.
pub struct Tracks {
    catalog: Catalog,
    templates: BTreeMap<String, Arc<Template>>,
    expectations: BTreeMap<ChallengeId, Expected>,
}

impl Tracks {
    /// The tracks in `catalog`.
    pub fn new(catalog: Catalog) -> Self {
        Self {
            catalog,
            templates: BTreeMap::new(),
            expectations: BTreeMap::new(),
        }
    }

    /// The tracks embedded in the binary. A track with bad content is left
    /// out; the content tests keep that from shipping.
    pub fn embedded() -> Self {
        Self::new(Catalog::embedded().0)
    }

    /// Every track, in order.
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Challenge `id` and its track.
    pub fn challenge(&self, id: &ChallengeId) -> Option<(&Track, &Challenge)> {
        self.catalog.challenge(id)
    }

    /// The template of the track with slug `slug`, built now if this is
    /// its first use.
    ///
    /// # Errors
    /// Returns the engine's error when there is no such track or its
    /// database can't be built.
    pub fn template(&mut self, slug: &str) -> Result<Arc<Template>, EngineError> {
        if let Some(template) = self.templates.get(slug) {
            return Ok(Arc::clone(template));
        }
        let track = self.catalog.track(slug).ok_or_else(|| EngineError::Open {
            target: slug.to_owned(),
            message: "there is no such track".to_owned(),
        })?;
        let template = Arc::new(Template::new(&track.schema, &track.seed)?);
        self.templates
            .insert(slug.to_owned(), Arc::clone(&template));
        Ok(template)
    }

    /// What a right answer to `id` looks like, as far as it is known.
    pub fn expected(&self, id: &ChallengeId) -> Expected {
        self.expectations.get(id).cloned().unwrap_or_default()
    }

    /// Whether `id`'s expectation still has to be worked out. It is then
    /// marked as being worked out, so it is asked for once.
    pub fn start_expecting(&mut self, id: &ChallengeId) -> bool {
        if self.expectations.contains_key(id) {
            return false;
        }
        self.expectations.insert(id.clone(), Expected::Pending);
        true
    }

    /// Records what is known of `id`'s expectation.
    pub fn set_expected(&mut self, id: &ChallengeId, expected: Expected) {
        self.expectations.insert(id.clone(), expected);
    }

    /// A session on a fresh copy of the template of `id`'s track.
    ///
    /// # Errors
    /// As for [`Tracks::template`], or when the copy can't be made.
    pub fn sandbox(&mut self, id: &ChallengeId) -> Result<Session, EngineError> {
        let copy = self.template(id.track())?.fresh()?;
        Ok(Session::start(Box::new(copy)))
    }
}

#[cfg(test)]
mod tests {
    use quarry_challenges::{
        catalog::{Catalog, ChallengeId},
        check::Expectation,
    };

    use super::Tracks;
    use crate::model::brief::Expected;

    #[test]
    fn an_expectation_is_asked_for_once_and_kept() {
        let mut tracks = Tracks::new(Catalog::default());
        let id = ChallengeId::new("t", "a");
        assert_eq!(tracks.expected(&id), Expected::Pending);
        assert!(tracks.start_expecting(&id));
        assert!(!tracks.start_expecting(&id));
        let ready = Expected::Ready(Expectation {
            columns: vec!["n".to_owned()],
            rows: 1,
        });
        tracks.set_expected(&id, ready.clone());
        assert_eq!(tracks.expected(&id), ready);
        assert!(!tracks.start_expecting(&id));
    }
}
