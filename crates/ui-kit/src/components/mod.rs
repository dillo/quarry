//! Reusable Aurora components. Each reads its colors from [`crate::Aurora`].

pub mod badge;
pub mod button;
pub mod kbd;
pub mod surface;

use std::rc::Rc;

use gpui_kit::{App, ClickEvent, Window};

/// A click callback shared by components.
pub type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
