//! Every challenge of every track; picking one opens it.

use gpui_kit::{App, Context, Window};
use quarry_challenges::catalog::ChallengeId;

use super::PaletteSource;
use crate::{model::palette::PaletteItem, views::workspace::Workspace};

/// The challenges, track by track, in play order.
pub struct Challenges;

impl PaletteSource for Challenges {
    fn id(&self) -> &'static str {
        "challenges"
    }

    fn title(&self) -> &'static str {
        "Challenges"
    }

    fn items(&self, workspace: &Workspace, _: &App) -> Vec<PaletteItem> {
        let mut items = Vec::new();
        for track in workspace.tracks().catalog().tracks() {
            let count = track.challenges.len();
            for (index, challenge) in track.challenges.iter().enumerate() {
                items.push(PaletteItem {
                    source: self.id(),
                    key: ChallengeId::new(&track.slug, &challenge.id).to_string(),
                    label: challenge.title.clone(),
                    detail: format!("{} · {} of {count}", track.title, index + 1),
                });
            }
        }
        items
    }

    fn pick(
        &self,
        key: &str,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        if let Some(id) = ChallengeId::parse(key) {
            workspace.open_challenge(&id, window, cx);
        }
    }
}
