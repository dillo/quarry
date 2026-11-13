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

use gpui_kit::{App, ClickEvent, Div, InteractiveElement, Stateful, Styled, Window};

use crate::palette::{Colors, Role};

/// A click callback shared by components.
pub type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// Makes an interactive element a keyboard stop. Tab reaches it, Enter and
/// Space click it (GPUI turns them into a click), and its border turns
/// [`Role::FocusRing`] while it has keyboard focus.
///
/// The element must already have a 1 px border (transparent when it has no
/// visible one), so gaining focus never changes its size.
pub fn keyboard_focusable(element: Stateful<Div>, colors: &Colors) -> Stateful<Div> {
    let ring = colors.get(Role::FocusRing);
    element
        .tab_index(0)
        .focus_visible(move |style| style.border_color(ring))
}
