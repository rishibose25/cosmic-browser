use wry::{WebView, WebViewBuilder};
use tao::window::Window;

pub struct BrowserEngine {
    // In a full impl, WebView is held here and positioned
    // to fill the content area using platform window embedding.
    // For now this drives navigation via IPC/URL changes.
    current_url: String,
    history: Vec<String>,
    history_pos: usize,
}

impl BrowserEngine {
    pub fn new() -> Self {
        Self {
            current_url: String::from("https://start.page"),
            history: vec![String::from("https://start.page")],
            history_pos: 0,
        }
    }

    pub fn navigate(&mut self, url: &str) {
        // Truncate forward history on new navigation
        self.history.truncate(self.history_pos + 1);
        self.history.push(url.to_string());
        self.history_pos = self.history.len() - 1;
        self.current_url = url.to_string();
        tracing::info!("Navigating to: {}", url);
        // TODO: call webview.load_url(url) once embedded
    }

    pub fn back(&mut self) {
        if self.history_pos > 0 {
            self.history_pos -= 1;
            self.current_url = self.history[self.history_pos].clone();
            tracing::info!("Back to: {}", self.current_url);
        }
    }

    pub fn forward(&mut self) {
        if self.history_pos + 1 < self.history.len() {
            self.history_pos += 1;
            self.current_url = self.history[self.history_pos].clone();
            tracing::info!("Forward to: {}", self.current_url);
        }
    }

    pub fn reload(&self) {
        tracing::info!("Reloading: {}", self.current_url);
        // TODO: call webview.reload() once embedded
    }

    pub fn current_url(&self) -> &str {
        &self.current_url
    }
}
