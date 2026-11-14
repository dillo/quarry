//! The strip of query tabs along the title bar, with a "+" for a new tab.

use gpui_kit::{
    AnyElement, Context, ElementId, IntoElement, ParentElement, Styled, Window,
    prelude::FluentBuilder, px,
};
use quarry_ui_kit::{
    components::tabs::{Tab, tab_bar},
    tokens::chrome,
};

use super::Workspace;
use crate::{model::tabs::TabId, shell::actions::NewTab};

/// Room left of the tabs for the traffic lights when the sidebar is hidden.
pub const TRAFFIC_LIGHTS_CLEARANCE: f32 = 78.0;

/// The element id of tab `id`'s button, as in "tab-3".
pub fn tab_element_id(id: TabId) -> ElementId {
    ElementId::Name(format!("tab-{id}").into())
}

impl Workspace {
    pub(super) fn tab_strip(&self, cx: &Context<Self>) -> AnyElement {
        let active = self.tabs.active_id();
        let tabs = self.tabs.tabs().iter().map(|tab| {
            let id = tab.id;
            Tab::new(tab_element_id(id), tab.title.clone())
                .selected(id == active)
                .on_click(cx.listener(move |workspace, _, window, cx| {
                    workspace.activate_tab(id, window, cx);
                }))
        });
        let new_tab = Tab::new("new-tab", "+").on_click(|_, window: &mut Window, cx| {
            window.dispatch_action(Box::new(NewTab), cx);
        });
        tab_bar()
            .h(px(chrome::TITLEBAR_HEIGHT))
            .when(!self.sidebar_open, |strip| {
                strip.pl(px(TRAFFIC_LIGHTS_CLEARANCE))
            })
            .children(tabs)
            .child(new_tab)
            .into_any_element()
    }
}
