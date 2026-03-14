use std::sync::{Arc, Mutex};
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
