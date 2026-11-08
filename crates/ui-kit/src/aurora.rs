//! The app-wide Aurora theme state: preference, appearance and cross-fade.

use std::time::Instant;

use gpui_kit::{App, Global, component::Theme, component::ThemeMode, component::ThemeRegistry};

use crate::{
    appearance::{Appearance, Preference},
    component_theme::{DARK_NAME, LIGHT_NAME, theme_set_json},
    palette::{Colors, Palette},
    tokens::motion,
    transition::Transition,
};

/// Why the theme could not be installed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ThemeError {
    /// `gpui-component` rejected the generated theme JSON.
    #[error("the Aurora theme could not be loaded: {0}")]
    Load(String),
}

/// Theme state, stored as a GPUI global.
pub struct Aurora {
    preference: Preference,
    appearance: Appearance,
    transition: Option<Transition>,
}

impl Global for Aurora {}

impl Aurora {
    /// Registers both themes with `gpui-component` and applies `preference`.
    /// Call once, after `gpui_kit::init`.
    ///
    /// # Errors
    /// Returns [`ThemeError::Load`] if the generated theme is rejected.
    pub fn install(preference: Preference, cx: &mut App) -> Result<(), ThemeError> {
        register_component_themes(cx)?;
        let appearance = preference.resolve(cx.window_appearance().into());
        cx.set_global(Self {
            preference,
            appearance,
            transition: None,
        });
        apply_component_mode(appearance, cx);
        Ok(())
    }

    /// The user's preference.
    ///
    /// # Panics
    /// Panics if [`Aurora::install`] has not been called.
    pub fn preference(cx: &App) -> Preference {
        cx.global::<Self>().preference
    }

    /// The appearance being shown (the target, during a cross-fade).
    ///
    /// # Panics
    /// Panics if [`Aurora::install`] has not been called.
    pub fn appearance(cx: &App) -> Appearance {
        cx.global::<Self>().appearance
    }

    /// The colors to draw right now.
    ///
    /// # Panics
    /// Panics if [`Aurora::install`] has not been called.
    pub fn colors(cx: &App) -> Colors {
        let aurora = cx.global::<Self>();
        aurora.transition.map_or_else(
            || palette_for(aurora.appearance).colors(),
            |fade| fade.colors(Instant::now()),
        )
    }

    /// Whether a cross-fade is running; views keep requesting frames while it is.
    ///
    /// # Panics
    /// Panics if [`Aurora::install`] has not been called.
    pub fn is_transitioning(cx: &App) -> bool {
        cx.global::<Self>()
            .transition
            .is_some_and(|fade| !fade.is_done(Instant::now()))
    }

    /// Changes the preference, cross-fading unless the system asks for reduced motion.
    ///
    /// # Panics
    /// Panics if [`Aurora::install`] has not been called.
    pub fn set_preference(preference: Preference, cx: &mut App) {
        let target = preference.resolve(cx.window_appearance().into());
        cx.global_mut::<Self>().preference = preference;
        Self::show(target, cx);
    }

    /// Re-reads the system appearance; call when macOS switches light/dark.
    ///
    /// # Panics
    /// Panics if [`Aurora::install`] has not been called.
    pub fn system_appearance_changed(cx: &mut App) {
        let target = Self::preference(cx).resolve(cx.window_appearance().into());
        Self::show(target, cx);
    }

    fn show(target: Appearance, cx: &mut App) {
        if Self::appearance(cx) == target {
            return;
        }
        let duration = if cx.reduce_motion() {
            std::time::Duration::ZERO
        } else {
            motion::THEME
        };
        let fade = Transition::new(
            Self::colors(cx),
            palette_for(target).colors(),
            Instant::now(),
            duration,
        );
        let aurora = cx.global_mut::<Self>();
        aurora.appearance = target;
        aurora.transition = Some(fade);
        apply_component_mode(target, cx);
    }
}

/// The palette for an appearance.
pub fn palette_for(appearance: Appearance) -> Palette {
    match appearance {
        Appearance::Dark => Palette::aurora(),
        Appearance::Light => Palette::dawn(),
    }
}

fn register_component_themes(cx: &mut App) -> Result<(), ThemeError> {
    ThemeRegistry::global_mut(cx)
        .load_themes_from_str(&theme_set_json())
        .map_err(|error| ThemeError::Load(error.to_string()))?;
    let themes = ThemeRegistry::global(cx).themes();
    let missing = |name: &str| ThemeError::Load(format!("theme {name} was not registered"));
    let dark = themes
        .get(DARK_NAME)
        .cloned()
        .ok_or_else(|| missing(DARK_NAME))?;
    let light = themes
        .get(LIGHT_NAME)
        .cloned()
        .ok_or_else(|| missing(LIGHT_NAME))?;
    let theme = Theme::global_mut(cx);
    theme.dark_theme = dark;
    theme.light_theme = light;
    Ok(())
}

fn apply_component_mode(appearance: Appearance, cx: &mut App) {
    let mode = match appearance {
        Appearance::Dark => ThemeMode::Dark,
        Appearance::Light => ThemeMode::Light,
    };
    Theme::change(mode, None, cx);
}
