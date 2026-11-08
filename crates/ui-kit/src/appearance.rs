//! Which theme to show: the user's preference resolved against the system.

/// A concrete appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    /// Aurora.
    Dark,
    /// Aurora Dawn.
    Light,
}

/// The user's theme setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Preference {
    /// Always Aurora (the default).
    #[default]
    Dark,
    /// Always Aurora Dawn.
    Light,
    /// Follow macOS.
    System,
}

impl Preference {
    /// The appearance to show when macOS is currently `system`.
    pub fn resolve(self, system: Appearance) -> Appearance {
        match self {
            Self::Dark => Appearance::Dark,
            Self::Light => Appearance::Light,
            Self::System => system,
        }
    }
}

impl From<gpui_kit::WindowAppearance> for Appearance {
    fn from(appearance: gpui_kit::WindowAppearance) -> Self {
        match appearance {
            gpui_kit::WindowAppearance::Dark | gpui_kit::WindowAppearance::VibrantDark => {
                Self::Dark
            }
            gpui_kit::WindowAppearance::Light | gpui_kit::WindowAppearance::VibrantLight => {
                Self::Light
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::WindowAppearance;

    use super::{Appearance, Preference};

    #[test]
    fn fixed_preferences_ignore_the_system() {
        for system in [Appearance::Dark, Appearance::Light] {
            assert_eq!(Preference::Dark.resolve(system), Appearance::Dark);
            assert_eq!(Preference::Light.resolve(system), Appearance::Light);
            assert_eq!(Preference::System.resolve(system), system);
        }
        assert_eq!(Preference::default(), Preference::Dark);
    }

    #[test]
    fn vibrant_system_appearances_map_to_their_base() {
        assert_eq!(
            Appearance::from(WindowAppearance::VibrantDark),
            Appearance::Dark
        );
        assert_eq!(Appearance::from(WindowAppearance::Dark), Appearance::Dark);
        assert_eq!(
            Appearance::from(WindowAppearance::VibrantLight),
            Appearance::Light
        );
        assert_eq!(Appearance::from(WindowAppearance::Light), Appearance::Light);
    }
}
