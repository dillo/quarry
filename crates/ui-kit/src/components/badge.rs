//! Small pill labels: concept tags, progress counts, statuses.

use gpui_kit::{
    App, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px,
};

use crate::{
    Aurora,
    palette::{Colors, Role},
    tokens::{radius, text},
};

/// A badge's color family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Violet, for concepts and accents.
    Accent,
    /// Teal, for success and progress.
    Success,
    /// Amber, for hints and warnings.
    Warning,
    /// Orange, for errors.
    Danger,
    /// Grey, for neutral metadata.
    Neutral,
}

impl Tone {
    /// `(background, text)` for this tone: the tone color at low opacity behind
    /// the full-strength tone color.
    pub fn colors(self, colors: &Colors) -> (Hsla, Hsla) {
        let ink = match self {
            Self::Accent => colors.get(Role::AccentText),
            Self::Success => colors.get(Role::Success),
            Self::Warning => colors.get(Role::Warning),
            Self::Danger => colors.get(Role::Danger),
            Self::Neutral => colors.get(Role::TextSecondary),
        };
        (ink.opacity(0.15), ink)
    }
}

/// A pill-shaped label.
#[derive(IntoElement)]
pub struct Badge {
    label: SharedString,
    tone: Tone,
}

impl Badge {
    /// A badge with `label` in `tone`.
    pub fn new(label: impl Into<SharedString>, tone: Tone) -> Self {
        Self {
            label: label.into(),
            tone,
        }
    }
}

impl RenderOnce for Badge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (background, ink) = self.tone.colors(&Aurora::colors(cx));
        div()
            .px(px(6.))
            .rounded(px(radius::PILL))
            .text_size(px(text::CAPTION))
            .bg(background)
            .text_color(ink)
            .child(self.label)
    }
}

#[cfg(test)]
mod tests {
    use super::Tone;
    use crate::palette::{Palette, Role};

    #[test]
    fn each_tone_draws_its_role_over_a_faint_wash_of_itself() {
        let colors = Palette::aurora().colors();
        let cases = [
            (Tone::Accent, Role::AccentText),
            (Tone::Success, Role::Success),
            (Tone::Warning, Role::Warning),
            (Tone::Danger, Role::Danger),
            (Tone::Neutral, Role::TextSecondary),
        ];
        for (tone, role) in cases {
            let (background, ink) = tone.colors(&colors);
            assert_eq!(ink, colors.get(role), "{tone:?}");
            assert_eq!(background, colors.get(role).opacity(0.15), "{tone:?}");
        }
    }
}
