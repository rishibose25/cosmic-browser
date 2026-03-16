use std::sync::{Arc, Mutex};
use cosmic::iced::{
    advanced::{
        layout, renderer,
        widget::{self, Widget},
        Layout, Shell,
    },
    mouse, Length, Rectangle, Size,
};

pub type BoundsState = Arc<Mutex<Option<Rectangle<f32>>>>;

pub struct WebViewPlaceholder<Message> {
    bounds_out: BoundsState,
    on_bounds_changed: Box<dyn Fn(Rectangle<f32>) -> Message>,
}

impl<Message> WebViewPlaceholder<Message> {
    pub fn new(
        bounds_out: BoundsState,
        on_bounds_changed: impl Fn(Rectangle<f32>) -> Message + 'static,
    ) -> Self {
        Self {
            bounds_out,
            on_bounds_changed: Box::new(on_bounds_changed),
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for WebViewPlaceholder<Message>
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
        // Intentionally empty — the WebView renders over this region.
    }

    // on_event fires every layout pass, letting us dispatch bounds changes
    // as proper iced messages rather than silently mutating shared state.
    fn on_event(
        &mut self,
        _tree: &mut widget::Tree,
        event: cosmic::iced::Event,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn cosmic::iced::advanced::Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> cosmic::iced::event::Status {
        let bounds = layout.bounds();

        let prev = self.bounds_out.lock().ok().and_then(|g| *g);
        let changed = prev.map(|p| {
            (p.x - bounds.x).abs() > 0.5
            || (p.y - bounds.y).abs() > 0.5
            || (p.width  - bounds.width ).abs() > 0.5
            || (p.height - bounds.height).abs() > 0.5
        }).unwrap_or(true);

        if changed {
            if let Ok(mut guard) = self.bounds_out.lock() {
                *guard = Some(bounds);
            }
            shell.publish((self.on_bounds_changed)(bounds));
        }

        cosmic::iced::event::Status::Ignored
    }
}

impl<'a, Message, Theme, Renderer> From<WebViewPlaceholder<Message>>
    for cosmic::Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Renderer: renderer::Renderer + 'a,
    Theme: 'a,
{
    fn from(w: WebViewPlaceholder<Message>) -> Self {
        Self::new(w)
    }
}
