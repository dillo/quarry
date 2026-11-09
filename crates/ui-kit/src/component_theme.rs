//! Generates `gpui-component`'s theme from the Aurora palettes, so the
//! editor, table and inputs share Quarry's colors (one source of truth).

use serde_json::{Value, json};

use crate::{
    color::{css, flatten},
    palette::{Palette, Role},
    tokens::{MONO_FONT, UI_FONT, text},
};

/// Registered name of the dark theme.
pub const DARK_NAME: &str = "Aurora";
/// Registered name of the light theme.
pub const LIGHT_NAME: &str = "Aurora Dawn";

/// `gpui-component` reads its radius as a whole number; kept equal to
/// [`radius::SM`] by a test.
const COMPONENT_RADIUS: u16 = 6;

/// The theme set (both themes) as `gpui-component` theme JSON.
pub fn theme_set_json() -> String {
    json!({
        "name": "Quarry Aurora",
        "themes": [
            config(DARK_NAME, "dark", &Palette::aurora()),
            config(LIGHT_NAME, "light", &Palette::dawn()),
        ],
    })
    .to_string()
}

fn config(name: &str, mode: &str, palette: &Palette) -> Value {
    json!({
        "name": name,
        "mode": mode,
        "font.family": UI_FONT,
        "font.size": text::BODY,
        "mono_font.family": MONO_FONT,
        "mono_font.size": text::BODY,
        "radius": COMPONENT_RADIUS,
        "colors": colors(palette),
        "highlight": highlight(palette),
    })
}

fn colors(p: &Palette) -> Value {
    let c = |role| css(p.hex(role));
    let surface = css(flatten(p.hex(Role::Panel), p.hex(Role::BackgroundMiddle)));
    json!({
        "background": surface,
        "foreground": c(Role::Text),
        "border": c(Role::Border),
        "input.border": c(Role::Border),
        "muted.background": c(Role::Hover),
        "muted.foreground": c(Role::TextMuted),
        "accent.background": c(Role::Hover),
        "accent.foreground": c(Role::Text),
        "primary.background": c(Role::AccentStart),
        "primary.foreground": c(Role::OnAccent),
        "ring": c(Role::AccentStart),
        "caret": c(Role::AccentText),
        "selection.background": c(Role::Selection),
        "list.active.background": c(Role::Selection),
        "list.active.border": c(Role::SelectionBorder),
        "table.background": "#00000000",
        "table.head.foreground": c(Role::TextSecondary),
        "table.row.border": c(Role::Border),
        "scrollbar.thumb.background": c(Role::Border),
        "danger.background": c(Role::Danger),
        "success.background": c(Role::Success),
        "warning.background": c(Role::Warning),
    })
}

fn highlight(p: &Palette) -> Value {
    let c = |role| css(p.hex(role));
    json!({
        "editor.background": "#00000000",
        "editor.foreground": c(Role::Text),
        "editor.line_number": c(Role::LineNumber),
        "editor.active_line_number": c(Role::TextSecondary),
        "editor.active_line.background": c(Role::ActiveLine),
        "syntax": {
            "keyword": { "color": c(Role::SyntaxKeyword) },
            "function": { "color": c(Role::SyntaxFunction) },
            "string": { "color": c(Role::SyntaxString) },
            "number": { "color": c(Role::SyntaxNumber) },
            "boolean": { "color": c(Role::SyntaxNumber) },
            "type": { "color": c(Role::SyntaxKeyword) },
            "comment": { "color": c(Role::SyntaxComment), "font_style": "italic" },
        },
    })
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{COMPONENT_RADIUS, DARK_NAME, LIGHT_NAME, theme_set_json};
    use crate::{
        color::css,
        palette::{Palette, Role},
        tokens::radius,
    };

    #[test]
    fn component_radius_matches_the_small_radius_token() {
        assert!((f32::from(COMPONENT_RADIUS) - radius::SM).abs() < f32::EPSILON);
    }

    fn themes() -> Result<Vec<Value>, serde_json::Error> {
        let set: Value = serde_json::from_str(&theme_set_json())?;
        Ok(set["themes"].as_array().cloned().unwrap_or_default())
    }

    #[test]
    fn defines_a_dark_and_a_light_theme() -> Result<(), serde_json::Error> {
        let themes = themes()?;
        let modes: Vec<(&str, &str)> = themes
            .iter()
            .map(|t| {
                (
                    t["name"].as_str().unwrap_or(""),
                    t["mode"].as_str().unwrap_or(""),
                )
            })
            .collect();
        assert_eq!(modes, [(DARK_NAME, "dark"), (LIGHT_NAME, "light")]);
        Ok(())
    }

    #[test]
    fn colors_come_from_the_palettes() -> Result<(), serde_json::Error> {
        let themes = themes()?;
        let text = |i: usize| {
            themes[i]["colors"]["foreground"]
                .as_str()
                .map(str::to_owned)
        };
        assert_eq!(text(0), Some(css(Palette::aurora().hex(Role::Text))));
        assert_eq!(text(1), Some(css(Palette::dawn().hex(Role::Text))));
        let keyword = &themes[0]["highlight"]["syntax"]["keyword"]["color"];
        assert_eq!(
            keyword.as_str(),
            Some(css(Palette::aurora().hex(Role::SyntaxKeyword)).as_str())
        );
        assert_eq!(
            themes[1]["highlight"]["syntax"]["comment"]["font_style"],
            "italic"
        );
        Ok(())
    }

    #[test]
    fn the_editor_surface_is_transparent_so_panels_show_through() -> Result<(), serde_json::Error> {
        for theme in themes()? {
            assert_eq!(theme["highlight"]["editor.background"], "#00000000");
            assert_eq!(theme["mono_font.family"], "Menlo");
        }
        Ok(())
    }
}
