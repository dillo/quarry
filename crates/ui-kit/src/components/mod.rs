//! Reusable Aurora components. Each reads its colors from [`crate::Aurora`].

pub mod badge;
pub mod button;
pub mod kbd;
pub mod segmented;
pub mod sidebar_item;
pub mod status_bar;
pub mod surface;
pub mod tabs;

use std::rc::Rc;

use gpui_kit::{App, ClickEvent, Window};

/// A click callback shared by components.
pub type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
