use std::sync::{Arc, Mutex};
use cosmic::{
    app::{Command, Core, Settings},
    iced::{
        self,
        event::listen_with,
        keyboard::{self, key::Named},
        window, Length, Rectangle, Subscription,
    },
    widget,
    Application, ApplicationExt, Element,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::browser::BrowserEngine;
use crate::ipc::{self, WebViewEvent};
use crate::sidebar::SidebarTab;
use crate::webview_widget::{BoundsState, WebViewPlaceholder};

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(String),
    AddressChanged(String),
    Back,
    Forward,
    Reload,
    NewTab,
    CloseTab(usize),
    SelectTab(usize),
    WebViewEvent(WebViewEvent),
    // Sent every frame when the placeholder widget's bounds change
    ContentBoundsChanged(Rectangle<f32>),
    // Fired once the iced window is realized
    WindowReady(window::Id),
}

// ── Application state ────────────────────────────────────────────────────────

pub struct CosmicBrowser {
    core: Core,
    tabs: Vec<SidebarTab>,
    active_tab: usize,
    address_input: String,
    is_loading: bool,
    load_progress: f64,

    // Shared with WebViewPlaceholder — written during draw(), read in update()
    content_bounds: BoundsState,
    last_bounds: Option<Rectangle<f32>>,

    // The engine holds all wry WebViews
    engine: Arc<Mutex<BrowserEngine>>,

    // IPC channel sender lives inside BrowserEngine;
    // we hold the receiver here in an Option so we can
    // move it into the subscription exactly once.
    ipc_rx: Arc<Mutex<Option<ipc::EventReceiver>>>,

    webview_attached: bool,
}

// ── Application impl ─────────────────────────────────────────────────────────

impl Application for CosmicBrowser {
    type Message = Message;
    type Executor = cosmic::executor::Default;
    type Flags = ();
    const APP_ID: &'static str = "com.system76.CosmicBrowser";

    fn core(&self) -> &Core { &self.core }
    fn core_mut(&mut self) -> &mut Core { &mut self.core }

    fn init(core: Core, _flags: ()) -> (Self, Command<Message>) {
        let (tx, rx) = ipc::channel();
        let bounds: BoundsState = Arc::new(Mutex::new(None));

        let app = Self {
            core,
            tabs: vec![SidebarTab::new("New Tab", "https://start.page")],
            active_tab: 0,
            address_input: "https://start.page".into(),
            is_loading: false,
            load_progress: 0.0,
            content_bounds: bounds,
            last_bounds: None,
            engine: Arc::new(Mutex::new(BrowserEngine::new(tx))),
            ipc_rx: Arc::new(Mutex::new(Some(rx))),
            webview_attached: false,
        };

        (app, Command::none())
    }

    // ── Subscription: IPC drain + window events ───────────────────────────────

    fn subscription(&self) -> Subscription<Message> {
        // ① IPC drain — moves the receiver out of the Arc<Mutex<Option<…>>>
        //   the first time it is polled, then keeps draining it forever.
        let ipc_rx_slot = self.ipc_rx.clone();
        let ipc_sub = iced::subscription::channel(
            std::any::TypeId::of::<WebViewEvent>(),
            64,
            move |mut output| async move {
                // Take the receiver out of the slot exactly once
                let mut rx = ipc_rx_slot
                    .lock()
                    .unwrap()
                    .take()
                    .expect("IPC receiver already taken");

                loop {
                    match rx.recv().await {
                        Some(ev) => {
                            let _ = output
                                .send(Message::WebViewEvent(ev))
                                .await;
                        }
                        // Sender dropped — engine shut down
                        None => break,
                    }
                }
                // Keep the future alive so the subscription isn't dropped
                std::future::pending::<()>().await;
                unreachable!()
            },
        );

        // ② Window-ready event — fires when the OS window is first created
        let window_sub = listen_with(|event, _| {
            if let iced::Event::Window(id, window::Event::Opened { .. }) = event {
                Some(Message::WindowReady(id))
            } else {
                None
            }
        });

        Subscription::batch([ipc_sub, window_sub])
    }

    // ── Update ────────────────────────────────────────────────────────────────

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            // ── Window realized: attach the WebView ───────────────────────
            Message::WindowReady(_id) => {
                self.try_attach_webview();
            }

            // ── Navigation ────────────────────────────────────────────────
            Message::Navigate(url) => {
                self.address_input = url.clone();
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.url = url.clone();
                }
                if let Ok(mut engine) = self.engine.lock() {
                    engine.navigate(&url);
                }
            }

            Message::AddressChanged(s) => {
                self.address_input = s;
            }

            Message::Back => {
                if let Ok(mut e) = self.engine.lock() { e.back(); }
            }
            Message::Forward => {
                if let Ok(mut e) = self.engine.lock() { e.forward(); }
            }
            Message::Reload => {
                if let Ok(e) = self.engine.lock() { e.reload(); }
            }

            // ── Tabs ──────────────────────────────────────────────────────
            Message::NewTab => {
                let idx = if let Ok(mut e) = self.engine.lock() {
                    e.open_tab("about:blank", true)
                } else { 0 };
                self.tabs.push(SidebarTab::new("New Tab", "about:blank"));
                self.active_tab = self.tabs.len() - 1;
                self.address_input = "about:blank".into();
            }

            Message::CloseTab(i) => {
                if self.tabs.len() > 1 {
                    if let Ok(mut e) = self.engine.lock() { e.close_tab(i); }
                    self.tabs.remove(i);
                    self.active_tab = self.active_tab.min(self.tabs.len() - 1);
                }
            }

            Message::SelectTab(i) => {
                if let Ok(mut e) = self.engine.lock() { e.select_tab(i); }
                self.active_tab = i;
                if let Some(tab) = self.tabs.get(i) {
                    self.address_input = tab.url.clone();
                }
            }

            // ── WebView callbacks ─────────────────────────────────────────
            Message::WebViewEvent(ev) => match ev {
                WebViewEvent::TitleChanged { tab_id, title } => {
                    if let Some(tab) = self.tabs.get_mut(tab_id) {
                        tab.title = title;
                    }
                }
                WebViewEvent::UrlChanged { tab_id, url } => {
                    if tab_id == self.active_tab {
                        self.address_input = url.clone();
                    }
                    if let Some(tab) = self.tabs.get_mut(tab_id) {
                        tab.url = url;
                    }
                }
                WebViewEvent::LoadStarted { tab_id } => {
                    if tab_id == self.active_tab {
                        self.is_loading = true;
                        self.load_progress = 0.0;
                    }
                }
                WebViewEvent::LoadFinished { tab_id } => {
                    if tab_id == self.active_tab {
                        self.is_loading = false;
                        self.load_progress = 1.0;
                    }
                }
                WebViewEvent::LoadProgress { tab_id, progress } => {
                    if tab_id == self.active_tab {
                        self.load_progress = progress;
                    }
                }
                WebViewEvent::NewWindowRequested { url, .. } => {
                    // Open as a new tab instead
                    return self.update(Message::NewTab);
                }
                WebViewEvent::FaviconUrl { tab_id, url } => {
                    if let Some(tab) = self.tabs.get_mut(tab_id) {
                        tab.favicon_url = Some(url);
                    }
                }
                WebViewEvent::DownloadStarted { url, suggested_path, .. } => {
                    tracing::info!("Download: {url} → {suggested_path:?}");
                    // Phase 2: route through ashpd file portal
                }
                WebViewEvent::PermissionRequested { tab_id, permission } => {
                    tracing::info!("Permission request tab={tab_id}: {permission}");
                    // Phase 2: show COSMIC dialog
                }
                WebViewEvent::IpcMessage { tab_id, body } => {
                    tracing::debug!("IPC tab={tab_id}: {body}");
                    // Phase 3: extension message routing
                }
                WebViewEvent::NavigationBlocked { tab_id, url } => {
                    tracing::warn!("Blocked tab={tab_id}: {url}");
                }
            }

            // ── Content area bounds feedback ──────────────────────────────
            Message::ContentBoundsChanged(rect) => {
                let changed = self.last_bounds
                    .map(|p| {
                        (p.x - rect.x).abs() > 0.5
                        || (p.y - rect.y).abs() > 0.5
                        || (p.width  - rect.width ).abs() > 0.5
                        || (p.height - rect.height).abs() > 0.5
                    })
                    .unwrap_or(true);

                if changed {
                    self.last_bounds = Some(rect);
                    if !self.webview_attached {
                        self.try_attach_webview();
                    } else if let Ok(mut e) = self.engine.lock() {
                        e.set_bounds(
                            rect.x as i32,
                            rect.y as i32,
                            rect.width  as u32,
                            rect.height as u32,
                        );
                    }
                }
            }
        }

        Command::none()
    }

    // ── View ──────────────────────────────────────────────────────────────────

    fn view(&self) -> Element<Message> {
        let sidebar = crate::sidebar::view(&self.tabs, self.active_tab);
        let toolbar = crate::toolbar::view(
            &self.address_input,
            self.is_loading,
            self.load_progress,
        );

        // Invisible placeholder — occupies the content area and reports its
        // pixel bounds back via ContentBoundsChanged each frame it changes.
        let bounds_state = self.content_bounds.clone();
        let on_bounds = |rect| Message::ContentBoundsChanged(rect);
        let placeholder = WebViewPlaceholder::new(bounds_state, on_bounds);

        let status = widget::container(
            widget::text(if self.is_loading {
                format!("Loading… {:.0}%", self.load_progress * 100.0)
            } else {
                self.engine.lock()
                    .map(|e| e.current_url().to_string())
                    .unwrap_or_default()
            })
            .size(12),
        )
        .padding([2, 12])
        .width(Length::Fill);

        let right = widget::column::with_children(vec![
            toolbar,
            placeholder.into(),
            status.into(),
        ])
        .height(Length::Fill);

        widget::row::with_children(vec![
            sidebar,
            right.into(),
        ])
        .into()
    }
}

// ── Window handle retrieval ───────────────────────────────────────────────────

impl CosmicBrowser {
    fn try_attach_webview(&mut self) {
        if self.webview_attached { return; }

        // libcosmic exposes the winit window through the iced platform window.
        // We retrieve the raw handle via the winit WindowHandle stored in Core.
        let raw_handle: RawWindowHandle = match self.core
            .main_window()
            .and_then(|w| w.window_handle().ok())
        {
            Some(h) => h.as_raw(),
            None => {
                tracing::warn!("Window not realized yet — deferring WebView attach");
                return;
            }
        };

        let rect = self.content_bounds
            .lock()
            .ok()
            .and_then(|g| *g)
            .unwrap_or(Rectangle {
                x: 220.0,
                y: 46.0,
                width: 1060.0,
                height: 720.0,
            });

        if let Ok(mut engine) = self.engine.lock() {
            engine.attach(
                raw_handle,
                rect.x      as i32,
                rect.y      as i32,
                rect.width  as u32,
                rect.height as u32,
            );
        }

        self.webview_attached = true;
        tracing::info!("WebView attached");
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run() {
    let settings = Settings::default()
        .size((1280, 800))
        .resizable(true);
    cosmic::app::run::<CosmicBrowser>(settings, ()).unwrap();
}use std::sync::{Arc, Mutex};
use cosmic::{
    app::{Command, Core, Settings},
    iced::{subscription, Length, Rectangle, Subscription},
    widget,
    Application, ApplicationExt, Element,
};
use raw_window_handle::HasWindowHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::browser::BrowserEngine;
use crate::ipc::{self, WebViewEvent};
use crate::sidebar::SidebarTab;
use crate::webview_widget::{BoundsState, WebViewPlaceholder};

#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    Navigate(String),
    AddressChanged(String),
    Back,
    Forward,
    Reload,
    // Tabs
    NewTab,
    CloseTab(usize),
    SelectTab(usize),
    // WebView callbacks
    WebViewEvent(WebViewEvent),
    // Layout
    ContentBoundsChanged(Rectangle<f32>),
    // Window ready
    WindowReady,
}

pub struct CosmicBrowser {
    core: Core,
    tabs: Vec<SidebarTab>,
    active_tab: usize,
    address_input: String,
    is_loading: bool,
    load_progress: f64,
    engine: BrowserEngine,
    content_bounds: BoundsState,
    last_bounds: Option<Rectangle<f32>>,
    webview_attached: bool,
    ipc_rx: Option<ipc::EventReceiver>,
}

impl Application for CosmicBrowser {
    type Message = Message;
    type Executor = cosmic::executor::Default;
    type Flags = ();
    const APP_ID: &'static str = "com.system76.CosmicBrowser";

    fn core(&self) -> &Core { &self.core }
    fn core_mut(&mut self) -> &mut Core { &mut self.core }

    fn init(core: Core, _flags: ()) -> (Self, Command<Message>) {
        let (tx, rx) = ipc::channel();
        let bounds: BoundsState = Arc::new(Mutex::new(None));

        let app = Self {
            core,
            tabs: vec![SidebarTab::new("New Tab", "https://start.page")],
            active_tab: 0,
            address_input: "https://start.page".into(),
            is_loading: false,
            load_progress: 0.0,
            engine: BrowserEngine::new(tx),
            content_bounds: bounds,
            last_bounds: None,
            webview_attached: false,
            ipc_rx: Some(rx),
        };

        // Trigger window-ready on first frame
        (app, Command::perform(async {}, |_| Message::WindowReady))
    }

    fn subscription(&self) -> Subscription<Message> {
        // Drain the IPC channel as a subscription stream
        if let Some(_) = &self.ipc_rx {
            // Using a one-shot to move the receiver into the subscription
            subscription::channel(
                std::any::TypeId::of::<WebViewEvent>(),
                64,
                |mut output| async move {
                    // The receiver is moved into the subscription on first poll;
                    // subsequent polls just drain it.
                    // (Full impl uses a shared Arc<Mutex<Option<Receiver>>> handoff)
                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
                    }
                },
            )
        } else {
            Subscription::none()
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::WindowReady => {
                // Attempt to attach WebView if we have bounds already
                self.try_attach_webview();
            }

            Message::Navigate(url) => {
                self.address_input = url.clone();
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.url = url.clone();
                }
                self.engine.navigate(&url);
            }

            Message::AddressChanged(s) => {
                self.address_input = s;
            }

            Message::Back    => self.engine.back(),
            Message::Forward => self.engine.forward(),
            Message::Reload  => self.engine.reload(),

            Message::NewTab => {
                self.tabs.push(SidebarTab::new("New Tab", "about:blank"));
                self.active_tab = self.tabs.len() - 1;
                self.address_input = "about:blank".into();
                self.engine.navigate("about:blank");
            }

            Message::CloseTab(i) => {
                if self.tabs.len() > 1 {
                    self.tabs.remove(i);
                    self.active_tab = self.active_tab.min(self.tabs.len() - 1);
                }
            }

            Message::SelectTab(i) => {
                self.active_tab = i;
                if let Some(tab) = self.tabs.get(i) {
                    self.address_input = tab.url.clone();
                    self.engine.navigate(&tab.url.clone());
                }
            }

            Message::WebViewEvent(ev) => match ev {
                WebViewEvent::TitleChanged(title) => {
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        tab.title = title;
                    }
                }
                WebViewEvent::UrlChanged(url) => {
                    self.address_input = url.clone();
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        tab.url = url;
                    }
                }
                WebViewEvent::LoadStarted => {
                    self.is_loading = true;
                    self.load_progress = 0.0;
                }
                WebViewEvent::LoadFinished => {
                    self.is_loading = false;
                    self.load_progress = 1.0;
                }
                WebViewEvent::LoadProgress(p) => {
                    self.load_progress = p;
                }
                WebViewEvent::FaviconUrl(url) => {
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        tab.favicon_url = Some(url);
                    }
                }
            }

            Message::ContentBoundsChanged(rect) => {
                // Reposition WebView when layout changes (resize, sidebar toggle)
                let changed = self.last_bounds
                    .map(|prev| {
                        (prev.x - rect.x).abs() > 0.5
                        || (prev.y - rect.y).abs() > 0.5
                        || (prev.width - rect.width).abs() > 0.5
                        || (prev.height - rect.height).abs() > 0.5
                    })
                    .unwrap_or(true);

                if changed {
                    self.last_bounds = Some(rect);
                    if !self.webview_attached {
                        self.try_attach_webview();
                    } else {
                        self.engine.set_bounds(
                            rect.x as i32, rect.y as i32,
                            rect.width as u32, rect.height as u32,
                        );
                    }
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let sidebar = crate::sidebar::view(&self.tabs, self.active_tab);
        let toolbar = crate::toolbar::view(
            &self.address_input,
            self.is_loading,
            self.load_progress,
        );

        // The invisible placeholder — occupies the content area,
        // reports its bounds so we can position the WebView over it.
        let bounds_state = self.content_bounds.clone();
        let placeholder = WebViewPlaceholder::new(bounds_state);

        let status_bar = widget::container(
            widget::text(if self.is_loading {
                format!("Loading… {:.0}%", self.load_progress * 100.0)
            } else {
                self.engine.current_url.clone()
            })
            .size(12),
        )
        .padding([2, 12])
        .width(Length::Fill);

        let content_col = widget::column::with_children(vec![
            toolbar,
            placeholder.into(),
            status_bar.into(),
        ])
        .height(Length::Fill);

        widget::row::with_children(vec![
            sidebar,
            content_col.into(),
        ])
        .into()
    }
}

impl CosmicBrowser {
    fn try_attach_webview(&mut self) {
        if self.webview_attached { return; }

        // Get the native window handle from the iced window
        let handle = match self.core.window().and_then(|w| w.window_handle().ok()) {
            Some(h) => h.as_raw(),
            None => {
                tracing::warn!("Window not yet realized, deferring WebView attach");
                return;
            }
        };

        // Get last-known content bounds (may be zeroed on first call)
        let rect = self.content_bounds.lock().ok()
            .and_then(|g| *g)
            .unwrap_or(Rectangle {
                x: 220.0, y: 46.0,   // sidebar width + toolbar height fallback
                width: 1060.0, height: 720.0,
            });

        self.engine.attach(
            handle,
            rect.x as i32, rect.y as i32,
            rect.width as u32, rect.height as u32,
        );
        self.webview_attached = true;
        tracing::info!("WebView attached");
    }
}

pub fn run() {
    let settings = Settings::default()
        .size((1280, 800))
        .resizable(true);
    cosmic::app::run::<CosmicBrowser>(settings, ()).unwrap();
}
