use wry::{WebView, WebViewBuilder, PageLoadEvent};
use raw_window_handle::RawWindowHandle;
use webkit2gtk::WebViewExt;
use glib::prelude::*;
use url::Url;

use crate::ipc::{EventSender, WebViewEvent};

// ── Per-tab WebView ───────────────────────────────────────────────────────────

pub struct TabWebView {
    pub webview: WebView,
    pub url: String,
    pub title: String,
    pub favicon_url: Option<String>,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub is_loading: bool,
    pub load_progress: f64,
}

impl TabWebView {
    fn new(
        window_handle: RawWindowHandle,
        x: i32, y: i32,
        width: u32, height: u32,
        initial_url: &str,
        sender: EventSender,
        tab_id: usize,
        visible: bool,
    ) -> Result<Self, wry::Error> {
        let tx_title  = sender.clone();
        let tx_nav    = sender.clone();
        let tx_load   = sender.clone();
        let tx_ipc    = sender.clone();
        let tx_newwin = sender.clone();
        let tx_dl     = sender.clone();
        let tx_perm   = sender.clone();

        #[cfg(debug_assertions)]
        let devtools = true;
        #[cfg(not(debug_assertions))]
        let devtools = false;

        let webview = WebViewBuilder::new()
            .with_url(initial_url)
            .with_user_agent(
                "Mozilla/5.0 (X11; Linux x86_64) \
                 AppleWebKit/605.1.15 (KHTML, like Gecko) \
                 CosmicBrowser/0.1 Safari/605.1.15"
            )
            .with_devtools(devtools)
            .with_clipboard(true)
            .with_accept_first_mouse(true)
            .with_initialization_script(
                &format!(r#"
                    window.__cosmicBrowser = {{
                        ipc: (msg) => window.ipc.postMessage(JSON.stringify(msg)),
                        tabId: {tab_id},
                    }};
                "#)
            )
            .with_title_changed_handler(move |title| {
                let _ = tx_title.send(WebViewEvent::TitleChanged { tab_id, title });
            })
            .with_navigation_handler(move |url| {
                if !is_allowed_scheme(&url) {
                    tracing::warn!("Blocked navigation to: {url}");
                    let _ = tx_nav.send(WebViewEvent::NavigationBlocked { tab_id, url });
                    return false;
                }
                let _ = tx_nav.send(WebViewEvent::UrlChanged { tab_id, url });
                true
            })
            .with_on_page_load_handler(move |event, _url| {
                match event {
                    PageLoadEvent::Started  =>
                        { let _ = tx_load.send(WebViewEvent::LoadStarted  { tab_id }); }
                    PageLoadEvent::Finished =>
                        { let _ = tx_load.send(WebViewEvent::LoadFinished { tab_id }); }
                }
            })
            .with_ipc_handler(move |request| {
                let _ = tx_ipc.send(WebViewEvent::IpcMessage {
                    tab_id,
                    body: request.body().to_string(),
                });
            })
            .with_new_window_requested_handler(move |url, _frame| {
                let _ = tx_newwin.send(WebViewEvent::NewWindowRequested { tab_id, url });
                false
            })
            .with_download_started_handler(move |url, path| {
                let _ = tx_dl.send(WebViewEvent::DownloadStarted {
                    tab_id,
                    url,
                    suggested_path: path.map(|p| p.to_string_lossy().to_string()),
                });
                false
            })
            .with_permission_handler(move |request| {
                let _ = tx_perm.send(WebViewEvent::PermissionRequested {
                    tab_id,
                    permission: format!("{:?}", request.permission()),
                });
                false
            })
            .with_bounds(wry::Rect {
                position: wry::dpi::LogicalPosition::new(x, y),
                size:     wry::dpi::LogicalSize::new(width, height),
            })
            .with_visible(visible)
            .build_as_child_of(window_handle)?;

        // ── webkit2gtk signals ────────────────────────────────────────────
        {
            let gtk_wv = webview.inner();

            // estimated-load-progress
            let tx_progress = sender.clone();
            gtk_wv.connect_notify(Some("estimated-load-progress"), move |wv, _| {
                let _ = tx_progress.send(WebViewEvent::LoadProgress {
                    tab_id,
                    progress: wv.estimated_load_progress(),
                });
            });

            // can-go-back and can-go-forward — both read both values
            let tx_back_fwd = sender.clone();
            gtk_wv.connect_notify(Some("can-go-back"), {
                let tx  = tx_back_fwd.clone();
                let wvr = gtk_wv.clone();
                move |_, _| {
                    let _ = tx.send(WebViewEvent::CanGoChanged {
                        tab_id,
                        back:    wvr.can_go_back(),
                        forward: wvr.can_go_forward(),
                    });
                }
            });
            gtk_wv.connect_notify(Some("can-go-forward"), {
                let tx  = tx_back_fwd.clone();
                let wvr = gtk_wv.clone();
                move |_, _| {
                    let _ = tx.send(WebViewEvent::CanGoChanged {
                        tab_id,
                        back:    wvr.can_go_back(),
                        forward: wvr.can_go_forward(),
                    });
                }
            });

            // uri — derive favicon URL from page origin
            let tx_favicon = sender.clone();
            gtk_wv.connect_notify(Some("uri"), move |wv, _| {
                if let Some(uri) = wv.uri() {
                    if let Ok(parsed) = Url::parse(uri.as_str()) {
                        let favicon = format!(
                            "{}://{}/favicon.ico",
                            parsed.scheme(),
                            parsed.host_str().unwrap_or("")
                        );
                        let _ = tx_favicon.send(WebViewEvent::FaviconUrl {
                            tab_id,
                            url: favicon,
                        });
                    }
                }
            });
        }

        Ok(Self {
            webview,
            url: initial_url.to_string(),
            title: "New Tab".to_string(),
            favicon_url: None,
            can_go_back: false,
            can_go_forward: false,
            is_loading: false,
            load_progress: 0.0,
        })
    }

    pub fn set_bounds(&self, x: i32, y: i32, width: u32, height: u32) {
        let _ = self.webview.set_bounds(wry::Rect {
            position: wry::dpi::LogicalPosition::new(x, y),
            size:     wry::dpi::LogicalSize::new(width, height),
        });
    }

    pub fn set_visible(&self, visible: bool) {
        let _ = self.webview.set_visible(visible);
    }

    pub fn navigate(&mut self, url: &str) {
        self.url = url.to_string();
        let _ = self.webview.load_url(url);
    }

    pub fn back(&self) {
        if self.can_go_back {
            let _ = self.webview.back();
        }
    }

    pub fn forward(&self) {
        if self.can_go_forward {
            let _ = self.webview.forward();
        }
    }

    pub fn reload(&self) {
        let _ = self.webview.reload();
    }

    pub fn zoom_in(&self) {
        let level = self.webview.zoom_level().unwrap_or(1.0);
        let _ = self.webview.set_zoom_level((level + 0.1).min(3.0));
    }

    pub fn zoom_out(&self) {
        let level = self.webview.zoom_level().unwrap_or(1.0);
        let _ = self.webview.set_zoom_level((level - 0.1).max(0.25));
    }

    pub fn zoom_reset(&self) {
        let _ = self.webview.set_zoom_level(1.0);
    }

    pub fn post_message(&self, json: &str) {
        let script = format!(
            "window.dispatchEvent(new MessageEvent('cosmicBrowser', \
             {{ data: {} }}));",
            json
        );
        let _ = self.webview.evaluate_script(&script);
    }
}

// ── Multi-tab engine ──────────────────────────────────────────────────────────

pub struct BrowserEngine {
    tabs: Vec<TabWebView>,
    active: usize,
    sender: EventSender,
    window_handle: Option<RawWindowHandle>,
    bounds: (i32, i32, u32, u32),
}

unsafe impl Send for BrowserEngine {}

impl BrowserEngine {
    pub fn new(sender: EventSender) -> Self {
        Self {
            tabs: Vec::new(),
            active: 0,
            sender,
            window_handle: None,
            bounds: (220, 46, 1060, 720),
        }
    }

    pub fn attach(
        &mut self,
        handle: RawWindowHandle,
        x: i32, y: i32,
        width: u32, height: u32,
    ) {
        self.window_handle = Some(handle);
        self.bounds = (x, y, width, height);
        self.open_tab("https://start.page", true);
    }

    pub fn is_attached(&self) -> bool {
        self.window_handle.is_some()
    }

    pub fn open_tab(&mut self, url: &str, activate: bool) -> usize {
        let handle = match self.window_handle {
            Some(h) => h,
            None => {
                tracing::error!("Cannot open tab: window not attached");
                return 0;
            }
        };

        let url = normalize_url(url);
        let tab_id = self.tabs.len();
        let (x, y, w, h) = self.bounds;

        if activate {
            for tab in &self.tabs {
                tab.set_visible(false);
            }
        }

        match TabWebView::new(
            handle, x, y, w, h,
            &url,
            self.sender.clone(),
            tab_id,
            activate,
        ) {
            Ok(tab) => {
                self.tabs.push(tab);
                if activate { self.active = tab_id; }
                tab_id
            }
            Err(e) => {
                tracing::error!("Failed to create WebView: {e}");
                0
            }
        }
    }

    pub fn close_tab(&mut self, idx: usize) {
        if self.tabs.len() <= 1 { return; }
        self.tabs.remove(idx);
        self.active = self.active.min(self.tabs.len() - 1);
        if let Some(tab) = self.tabs.get(self.active) {
            tab.set_visible(true);
        }
    }

    pub fn select_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() { return; }
        if let Some(prev) = self.tabs.get(self.active) {
            prev.set_visible(false);
        }
        self.active = idx;
        if let Some(tab) = self.tabs.get(idx) {
            tab.set_visible(true);
        }
    }

    pub fn set_bounds(&mut self, x: i32, y: i32, width: u32, height: u32) {
        self.bounds = (x, y, width, height);
        for tab in &self.tabs {
            tab.set_bounds(x, y, width, height);
        }
    }

    pub fn navigate(&mut self, url: &str) {
        let url = normalize_url(url);
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.navigate(&url);
        }
    }

    pub fn back(&mut self) {
        if let Some(tab) = self.tabs.get(self.active) { tab.back(); }
    }

    pub fn forward(&mut self) {
        if let Some(tab) = self.tabs.get(self.active) { tab.forward(); }
    }

    pub fn reload(&self) {
        if let Some(tab) = self.tabs.get(self.active) { tab.reload(); }
    }

    pub fn zoom_in(&self) {
        if let Some(tab) = self.tabs.get(self.active) { tab.zoom_in(); }
    }

    pub fn zoom_out(&self) {
        if let Some(tab) = self.tabs.get(self.active) { tab.zoom_out(); }
    }

    pub fn zoom_reset(&self) {
        if let Some(tab) = self.tabs.get(self.active) { tab.zoom_reset(); }
    }

    pub fn current_url(&self) -> &str {
        self.tabs.get(self.active)
            .map(|t| t.url.as_str())
            .unwrap_or("about:blank")
    }

    pub fn active_tab(&self) -> Option<&TabWebView> {
        self.tabs.get(self.active)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut TabWebView> {
        self.tabs.get_mut(self.active)
    }

    pub fn tab_mut(&mut self, idx: usize) -> Option<&mut TabWebView> {
        self.tabs.get_mut(idx)
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn update_can_go(&mut self, tab_id: usize, back: bool, forward: bool) {
        if let Some(tab) = self.tabs.get_mut(tab_id) {
            tab.can_go_back    = back;
            tab.can_go_forward = forward;
        }
    }
}

// ── URL normalisation ─────────────────────────────────────────────────────────

fn normalize_url(input: &str) -> String {
    if matches!(
        input.split(':').next().unwrap_or(""),
        "about" | "file" | "data" | "view-source" | "blob"
    ) {
        return input.to_string();
    }

    if let Ok(u) = Url::parse(input) {
        if u.scheme() == "http" || u.scheme() == "https" {
            return input.to_string();
        }
    }

    if input.starts_with("localhost")
        || input.starts_with("127.")
        || input.starts_with("[::1]")
    {
        return format!("http://{input}");
    }

    let looks_like_host = input.contains('.')
        && !input.contains(' ')
        && !input.contains('?')
        && !input.starts_with('.');

    if looks_like_host {
        return format!("https://{input}");
    }

    format!(
        "https://search.brave.com/search?q={}",
        urlencoding::encode(input)
    )
}

// ── Scheme allowlist ──────────────────────────────────────────────────────────

fn is_allowed_scheme(url: &str) -> bool {
    matches!(
        url.split(':').next().unwrap_or(""),
        "http" | "https" | "about" | "file" | "data" | "blob" | "view-source"
    )
}
