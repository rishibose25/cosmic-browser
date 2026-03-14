use std::sync::{Arc, Mutex};
use wry::{WebView, WebViewBuilder};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use crate::ipc::{EventSender, WebViewEvent};
use url::Url;

pub struct BrowserEngine {
    webview: Option<WebView>,
    history: Vec<String>,
    history_pos: usize,
    pub current_url: String,
    sender: EventSender,
}

impl BrowserEngine {
    pub fn new(sender: EventSender) -> Self {
        Self {
            webview: None,
            history: vec!["about:blank".into()],
            history_pos: 0,
            current_url: "about:blank".into(),
            sender,
        }
    }

    /// Called once the iced window is realized and we have a native handle + content rect.
    pub fn attach(
        &mut self,
        window_handle: RawWindowHandle,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) {
        let tx = self.sender.clone();
        let tx2 = self.sender.clone();
        let tx3 = self.sender.clone();
        let tx4 = self.sender.clone();

        let webview = WebViewBuilder::new()
            .with_url(&self.current_url)
            // Feed title changes back to the shell
            .with_title_changed_handler(move |title| {
                let _ = tx.send(WebViewEvent::TitleChanged(title));
            })
            // Feed URL changes (navigation, redirects)
            .with_navigation_handler(move |url| {
                let _ = tx2.send(WebViewEvent::UrlChanged(url.clone()));
                true // allow navigation
            })
            // Load progress
            .with_on_page_load_handler(move |event, _url| {
                use wry::PageLoadEvent;
                match event {
                    PageLoadEvent::Started  => { let _ = tx3.send(WebViewEvent::LoadStarted); }
                    PageLoadEvent::Finished => { let _ = tx4.send(WebViewEvent::LoadFinished); }
                }
            })
            .with_bounds(wry::Rect {
                position: wry::dpi::LogicalPosition::new(x, y),
                size: wry::dpi::LogicalSize::new(width, height),
            })
            .build_as_child_of(window_handle) // hole-punch: parent = iced window
            .expect("Failed to create WebView");

        self.webview = Some(webview);
        tracing::info!("WebView attached at ({x},{y}) {width}x{height}");
    }

    /// Reposition/resize the WebView when the content area changes (window resize, sidebar toggle).
    pub fn set_bounds(&self, x: i32, y: i32, width: u32, height: u32) {
        if let Some(wv) = &self.webview {
            let _ = wv.set_bounds(wry::Rect {
                position: wry::dpi::LogicalPosition::new(x, y),
                size: wry::dpi::LogicalSize::new(width, height),
            });
        }
    }

    pub fn navigate(&mut self, url: &str) {
        let url = normalize_url(url);
        self.history.truncate(self.history_pos + 1);
        self.history.push(url.clone());
        self.history_pos = self.history.len() - 1;
        self.current_url = url.clone();
        if let Some(wv) = &self.webview {
            let _ = wv.load_url(&url);
        }
    }

    pub fn back(&mut self) {
        if self.history_pos > 0 {
            self.history_pos -= 1;
            self.current_url = self.history[self.history_pos].clone();
            if let Some(wv) = &self.webview {
                let _ = wv.evaluate_script("history.back()");
            }
        }
    }

    pub fn forward(&mut self) {
        if self.history_pos + 1 < self.history.len() {
            self.history_pos += 1;
            self.current_url = self.history[self.history_pos].clone();
            if let Some(wv) = &self.webview {
                let _ = wv.evaluate_script("history.forward()");
            }
        }
    }

    pub fn reload(&self) {
        if let Some(wv) = &self.webview {
            let _ = wv.evaluate_script("location.reload()");
        }
    }

    pub fn is_attached(&self) -> bool {
        self.webview.is_some()
    }
}

/// Turn bare search queries and schemeless hosts into proper URLs.
fn normalize_url(input: &str) -> String {
    if input.starts_with("about:") { return input.to_string(); }
    if Url::parse(input).is_ok() { return input.to_string(); }
    // Has a dot but no scheme → assume https
    if input.contains('.') && !input.contains(' ') {
        return format!("https://{input}");
    }
    // Treat as a search query
    let query = urlencoding::encode(input);
    format!("https://search.brave.com/search?q={query}")
}
