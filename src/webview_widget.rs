use cosmic::iced::{
    advanced::{
        layout, renderer,
        widget::{self, Widget},
        Layout, Shell,
    },
    event, mouse, Length, Rectangle, Size,
};
use std::sync::{Arc, Mutex};

/// Shared state: the last-known absolute bounds of the content area.
pub type BoundsState = Arc<Mutex<Option<Rectangle<f32>>>>;

pub struct WebViewPlaceholder {
    bounds_out: BoundsState,
}

impl WebViewPlaceholder {
    pub fn new(bounds_out: BoundsState) -> Self {
        Self { bounds_out }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for WebViewPlaceholder
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn draw(
        &self,
        _tree: &widget::Tree,
        _renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        // Transparent — the WebView renders over this area.
        // Capture bounds so app.rs can reposition the WebView.
        let bounds = layout.bounds();
        if let Ok(mut guard) = self.bounds_out.lock() {
            *guard = Some(bounds);
        }
    }
}

impl<'a, Message, Theme, Renderer> From<WebViewPlaceholder>
    for cosmic::Element<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn from(w: WebViewPlaceholder) -> Self {
        Self::new(w)
    }
}
