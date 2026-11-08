//! The Aurora (dark) and Aurora Dawn (light) palettes, by semantic role.

use gpui_kit::Hsla;

use crate::color;

/// What a color is for. Components ask for roles, never raw colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Window gradient, first stop.
    BackgroundStart,
    /// Window gradient, middle stop.
    BackgroundMiddle,
    /// Window gradient, last stop.
    BackgroundEnd,
    /// Sidebar and inspector glass.
    Sidebar,
    /// Floating panel glass (editor, results).
    Panel,
    /// Panel and divider borders.
    Border,
    /// Hover wash on interactive items.
    Hover,
    /// Primary text.
    Text,
    /// Secondary text (labels, metadata).
    TextSecondary,
    /// Muted text (captions, placeholders).
    TextMuted,
    /// Accent gradient, first stop.
    AccentStart,
    /// Accent gradient, last stop.
    AccentEnd,
    /// Accent-colored text and icons.
    AccentText,
    /// Text on the accent gradient.
    OnAccent,
    /// Selected-item wash.
    Selection,
    /// Selected-item border.
    SelectionBorder,
    /// Success (correct answer, connected).
    Success,
    /// Warning (hints, amber states).
    Warning,
    /// Errors and destructive actions.
    Danger,
    /// SQL keywords.
    SyntaxKeyword,
    /// SQL functions.
    SyntaxFunction,
    /// String literals.
    SyntaxString,
    /// Numeric literals.
    SyntaxNumber,
    /// Comments.
    SyntaxComment,
    /// Editor line numbers.
    LineNumber,
    /// Editor current-line wash.
    ActiveLine,
}

impl Role {
    /// Every role, in declaration order.
    pub const ALL: [Self; 26] = [
        Self::BackgroundStart,
        Self::BackgroundMiddle,
        Self::BackgroundEnd,
        Self::Sidebar,
        Self::Panel,
        Self::Border,
        Self::Hover,
        Self::Text,
        Self::TextSecondary,
        Self::TextMuted,
        Self::AccentStart,
        Self::AccentEnd,
        Self::AccentText,
        Self::OnAccent,
        Self::Selection,
        Self::SelectionBorder,
        Self::Success,
        Self::Warning,
        Self::Danger,
        Self::SyntaxKeyword,
        Self::SyntaxFunction,
        Self::SyntaxString,
        Self::SyntaxNumber,
        Self::SyntaxComment,
        Self::LineNumber,
        Self::ActiveLine,
    ];

    fn index(self) -> usize {
        self as usize
    }
}

const AURORA: [(Role, u32); 26] = [
    (Role::BackgroundStart, 0x1b15_30ff),
    (Role::BackgroundMiddle, 0x0e14_24ff),
    (Role::BackgroundEnd, 0x0b1b_22ff),
    (Role::Sidebar, 0xffff_ff09),
    (Role::Panel, 0xffff_ff08),
    (Role::Border, 0xffff_ff10),
    (Role::Hover, 0xffff_ff0f),
    (Role::Text, 0xe6e4_f0ff),
    (Role::TextSecondary, 0xe6e4_f0c1),
    (Role::TextMuted, 0xe6e4_f08c),
    (Role::AccentStart, 0x8b6c_ffff),
    (Role::AccentEnd, 0x5b8c_ffff),
    (Role::AccentText, 0xcbb8_ffff),
    (Role::OnAccent, 0xffff_ffff),
    (Role::Selection, 0x8b6c_ff47),
    (Role::SelectionBorder, 0xa087_ff40),
    (Role::Success, 0x5eea_d4ff),
    (Role::Warning, 0xfbbf_24ff),
    (Role::Danger, 0xfcae_6fff),
    (Role::SyntaxKeyword, 0xcbb8_ffff),
    (Role::SyntaxFunction, 0x5eea_d4ff),
    (Role::SyntaxString, 0xfbbf_77ff),
    (Role::SyntaxNumber, 0xf9a8_d4ff),
    (Role::SyntaxComment, 0xe6e4_f080),
    (Role::LineNumber, 0xe6e4_f071),
    (Role::ActiveLine, 0xffff_ff0a),
];

const DAWN: [(Role, u32); 26] = [
    (Role::BackgroundStart, 0xf3ee_ffff),
    (Role::BackgroundMiddle, 0xeef3_ffff),
    (Role::BackgroundEnd, 0xe9f8_f6ff),
    (Role::Sidebar, 0xffff_ff73),
    (Role::Panel, 0xffff_ffb8),
    (Role::Border, 0x6e5a_c81a),
    (Role::Hover, 0x6e5a_c80f),
    (Role::Text, 0x2420_3aff),
    (Role::TextSecondary, 0x2420_3ab3),
    (Role::TextMuted, 0x2420_3a99),
    (Role::AccentStart, 0x8b6c_ffff),
    (Role::AccentEnd, 0x5b8c_ffff),
    (Role::AccentText, 0x593f_bcff),
    (Role::OnAccent, 0xffff_ffff),
    (Role::Selection, 0x8b6c_ff2e),
    (Role::SelectionBorder, 0x8b6c_ff40),
    (Role::Success, 0x0c61_5bff),
    (Role::Warning, 0x8843_08ff),
    (Role::Danger, 0x9833_09ff),
    (Role::SyntaxKeyword, 0x593f_bcff),
    (Role::SyntaxFunction, 0x0c61_5bff),
    (Role::SyntaxString, 0x8843_08ff),
    (Role::SyntaxNumber, 0xb628_78ff),
    (Role::SyntaxComment, 0x6b65_85ff),
    (Role::LineNumber, 0x2420_3a82),
    (Role::ActiveLine, 0x8b6c_ff0f),
];

/// A theme's colors as `0xRRGGBBAA` values, one per [`Role`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    colors: [u32; Role::ALL.len()],
}

impl Palette {
    /// Aurora, the dark default theme.
    pub fn aurora() -> Self {
        Self::from_pairs(&AURORA)
    }

    /// Aurora Dawn, the light theme.
    pub fn dawn() -> Self {
        Self::from_pairs(&DAWN)
    }

    /// The `0xRRGGBBAA` value for `role`.
    pub fn hex(&self, role: Role) -> u32 {
        self.colors[role.index()]
    }

    /// The renderable colors.
    pub fn colors(&self) -> Colors {
        Colors {
            colors: self.colors.map(color::hsla),
        }
    }

    fn from_pairs(pairs: &[(Role, u32); Role::ALL.len()]) -> Self {
        let mut colors = [0; Role::ALL.len()];
        for (role, value) in pairs {
            colors[role.index()] = *value;
        }
        Self { colors }
    }
}

/// Renderable colors, possibly mid-way between two palettes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    colors: [Hsla; Role::ALL.len()],
}

impl Colors {
    /// The color for `role`.
    pub fn get(&self, role: Role) -> Hsla {
        self.colors[role.index()]
    }

    /// The colors `t` of the way from `from` to `to` (0 = `from`, 1 = `to`).
    pub fn mix(from: &Self, to: &Self, t: f32) -> Self {
        let mut colors = from.colors;
        for (index, color) in colors.iter_mut().enumerate() {
            *color = color::mix(*color, to.colors[index], t);
        }
        Self { colors }
    }
}

#[cfg(test)]
mod tests {
    use super::{AURORA, Colors, DAWN, Palette, Role};
    use crate::color;

    #[test]
    fn every_role_is_defined_exactly_once_in_each_palette() {
        for table in [AURORA, DAWN] {
            for (index, role) in Role::ALL.iter().enumerate() {
                assert_eq!(table[index].0, *role, "table order must follow Role::ALL");
                assert_eq!(role.index(), index);
            }
        }
    }

    #[test]
    fn palettes_read_back_their_values() {
        assert_eq!(Palette::aurora().hex(Role::Text), 0xe6e4_f0ff);
        assert_eq!(Palette::dawn().hex(Role::Text), 0x2420_3aff);
        assert_eq!(
            Palette::aurora().colors().get(Role::OnAccent),
            color::hsla(0xffff_ffff)
        );
    }

    #[test]
    fn mixing_moves_every_role_between_palettes() {
        let (dark, light) = (Palette::aurora().colors(), Palette::dawn().colors());
        assert_eq!(Colors::mix(&dark, &light, 0.0), dark);
        assert_eq!(Colors::mix(&dark, &light, 1.0), light);
        let half = Colors::mix(&dark, &light, 0.5);
        for role in Role::ALL {
            let expected = color::mix(dark.get(role), light.get(role), 0.5);
            assert_eq!(half.get(role), expected, "{role:?}");
        }
    }
}
