//! The line under a page's groups: a caption and the link buttons that go with it, drawn by the
//! `FooterLine` component.

use std::rc::Rc;

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::ui::{FooterLineProps, FooterLink, Handler, Look};

/// A caption and its links; add each with [`link`](Self::link).
#[derive(IntoElement)]
pub struct Footer {
    text: SharedString,
    links: Vec<FooterLink>,
}

pub fn footer(text: impl Into<SharedString>) -> Footer {
    Footer { text: text.into(), links: Vec::new() }
}

impl Footer {
    /// A link button after the caption.
    pub fn link(
        mut self,
        name: impl Into<SharedString>,
        pick: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        let pick: Handler = Rc::new(pick);
        self.links.push(FooterLink { key: self.links.len(), name: name.into(), pick: Some(pick) });
        self
    }
}

impl RenderOnce for Footer {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        FooterLineProps { text: self.text, links: self.links }.render(&Look::of(cx))
    }
}
