use std::sync::{Arc, Mutex};
use cosmic::{
    app::{Command, Core, Settings},
    iced::{
        self,
        event::listen_with,
        window,
        Length, Rectangle, Subscription,
    },
    widget,
    Application, ApplicationExt, Element,
};
use raw_window_handle::HasWindowHandle;

use crate::browser::BrowserEngine;
use crate::ipc::{self, WebViewEvent};
use crate::settings::BrowserSettings;
use crate::settings_ui::SettingsMessage;
use crate::sidebar::SidebarTab;
use crate::webview_widget::{BoundsState, WebViewPlaceholder};

// ── Messages ──────────────────────────────────────────────────────────────────

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
    OpenSettings,
    CloseSettings,
    SettingsAction(SettingsMessage),
    WebViewEvent(WebViewEvent),
    ContentBoundsChanged(Rectangle<f32>),
    WindowReady(window::Id),
}

// ── State ─────────────────────────────────────────────────────────────────────

pub struct CosmicBrowser {
    core: Core,
    tabs: Vec<SidebarTab>,
    active_tab: usize,
    address_input: String,
    is_loading: bool,
    load_progress: f64,
    content_bounds: BoundsState,
    last_bounds: Option<Rectangle<f32>>,
    engine: Arc<Mutex<BrowserEngine>>,
    ipc_rx: Arc<Mutex<Option<ipc::EventReceiver>>>,
    webview_attached: bool,
    settings: BrowserSettings,
    show_settings: bool,
}

// ── Application ───────────────────────────────────────────────────────────────

impl Application for CosmicBrowser {
    type Message = Message;
    type Executor = cosmic::executor::Default;
    type Flags = ();
    const APP_ID: &'static str = "com.system76.CosmicBrowser";

    fn core(&self) -> &Core { &self.core }
    fn core_mut(&mut self) -> &mut Core { &mut self.core }

    fn init(core: Core, _flags: ()) -> (Self, Command<Message>) {
        let (tx, rx) = ipc::channel();
        let settings = BrowserSettings::load();

        let app = Self {
            core,
            tabs: vec![SidebarTab::new("New Tab", &settings.general.homepage)],
            active_tab: 0,
            address_input: settings.general.homepage.clone(),
            is_loading: false,
            load_progress: 0.0,
            content_bounds: Arc::new(Mutex::new(None)),
            last_bounds: None,
            engine: Arc::new(Mutex::new(BrowserEngine::new(tx))),
            ipc_rx: Arc::new(Mutex::new(Some(rx))),
            webview_attached: false,
            settings,
            show_settings: false,
        };

        (app, Command::none())
    }

    // ── Subscriptions ─────────────────────────────────────────────────────────

    fn subscription(&self) -> Subscription<Message> {
        let ipc_rx_slot = self.ipc_rx.clone();

        let ipc_sub = iced::subscription::channel(
            std::any::TypeId::of::<WebViewEvent>(),
            64,
            move |mut output| async move {
                let mut rx = ipc_rx_slot
                    .lock()
                    .unwrap()
                    .take()
                    .expect("IPC receiver already taken");

                loop {
                    match rx.recv().await {
                        Some(ev) => { let _ = output.send(Message::WebViewEvent(ev)).await; }
                        None     => break,
                    }
                }

                std::future::pending::<()>().await;
                unreachable!()
            },
        );

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
            Message::WindowReady(_) => {
                self.try_attach_webview();
            }

            Message::Navigate(url) => {
                self.address_input = url.clone();
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.url = url.clone();
                }
                if let Ok(mut e) = self.engine.lock() {
                    e.navigate(&url, &self.settings.search);
                }
            }

            Message::AddressChanged(s) => {
                self.address_input = s;
            }

            Message::Back    => { if let Ok(mut e) = self.engine.lock() { e.back(); } }
            Message::Forward => { if let Ok(mut e) = self.engine.lock() { e.forward(); } }
            Message::Reload  => { if let Ok(e)     = self.engine.lock() { e.reload(); } }

            Message::NewTab => {
                let url = self.settings.general.new_tab_url.clone();
                self.tabs.push(SidebarTab::new("New Tab", &url));
                self.active_tab = self.tabs.len() - 1;
                self.address_input = url.clone();
                if let Ok(mut e) = self.engine.lock() {
                    e.open_tab(&url, true);
                }
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

            Message::OpenSettings => {
                self.show_settings = true;
            }

            Message::CloseSettings => {
                self.show_settings = false;
            }

            Message::SettingsAction(msg) => {
                match msg {
                    SettingsMessage::HomepageChanged(v)         => self.settings.general.homepage = v,
                    SettingsMessage::NewTabUrlChanged(v)        => self.settings.general.new_tab_url = v,
                    SettingsMessage::RestoreSessionToggled(v)   => self.settings.general.restore_session = v,
                    SettingsMessage::JavascriptToggled(v)       => self.settings.privacy.javascript_enabled = v,
                    SettingsMessage::CookiesToggled(v)          => self.settings.privacy.cookies_enabled = v,
                    SettingsMessage::BlockThirdPartyToggled(v)  => self.settings.privacy.block_third_party_cookies = v,
                    SettingsMessage::DoNotTrackToggled(v)       => self.settings.privacy.do_not_track = v,
                    SettingsMessage::ClearOnCloseToggled(v)     => self.settings.privacy.clear_on_close = v,
                    SettingsMessage::ThemeSelected(v)           => self.settings.appearance.theme = v,
                    SettingsMessage::SidebarPositionSelected(v) => self.settings.appearance.sidebar_position = v,
                    SettingsMessage::StatusBarToggled(v)        => self.settings.appearance.show_status_bar = v,
                    SettingsMessage::SearchEngineSelected(v)    => self.settings.search.engine = v,
                    SettingsMessage::CustomSearchUrlChanged(v)  => self.settings.search.custom_url = Some(v),
                    SettingsMessage::SuggestToggled(v)          => self.settings.search.suggest = v,
                    SettingsMessage::Save => {
                        self.settings.save();
                        self.show_settings = false;
                    }
                    SettingsMessage::Close => {
                        self.settings = BrowserSettings::load();
                        self.show_settings = false;
                    }
                }
            }

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
                WebViewEvent::FaviconUrl { tab_id, url } => {
                    if let Some(tab) = self.tabs.get_mut(tab_id) {
                        tab.favicon_url = Some(url);
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
                WebViewEvent::CanGoChanged { tab_id, back, forward } => {
                    if let Ok(mut e) = self.engine.lock() {
                        e.update_can_go(tab_id, back, forward);
                    }
                }
                WebViewEvent::NewWindowRequested { url, .. } => {
                    let url = url.clone();
                    self.tabs.push(SidebarTab::new("New Tab", &url));
                    self.active_tab = self.tabs.len() - 1;
                    self.address_input = url.clone();
                    if let Ok(mut e) = self.engine.lock() {
                        e.open_tab(&url, true);
                    }
                }
                WebViewEvent::DownloadStarted { url, suggested_path, .. } => {
                    tracing::info!("Download: {url} → {suggested_path:?}");
                }
                WebViewEvent::PermissionRequested { tab_id, permission } => {
                    tracing::info!("Permission request tab={tab_id}: {permission}");
                }
                WebViewEvent::IpcMessage { tab_id, body } => {
                    tracing::debug!("IPC tab={tab_id}: {body}");
                }
                WebViewEvent::NavigationBlocked { tab_id, url } => {
                    tracing::warn!("Blocked tab={tab_id}: {url}");
                }
            }

            Message::ContentBoundsChanged(rect) => {
                let changed = self.last_bounds
                    .map(|p| {
                        (p.x      - rect.x     ).abs() > 0.5
                        || (p.y      - rect.y     ).abs() > 0.5
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
                            rect.x      as i32,
                            rect.y      as i32,
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
        if self.show_settings {
            return crate::settings_ui::view(&self.settings)
                .map(Message::SettingsAction);
        }

        let sidebar = crate::sidebar::view(&self.tabs, self.active_tab);
        let toolbar = crate::toolbar::view(
            &self.address_input,
            self.is_loading,
            self.load_progress,
        );

        let placeholder = WebViewPlaceholder::new(
            self.content_bounds.clone(),
            Message::ContentBoundsChanged,
        );

        let status = if self.settings.appearance.show_status_bar {
            Some(
                widget::container(
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
                .width(Length::Fill)
                .into(),
            )
        } else {
            None
        };

        let mut right_col = widget::column::with_children(vec![
            toolbar,
            placeholder.into(),
        ]);

        if let Some(bar) = status {
            right_col = right_col.push(bar);
        }

        let right = right_col.height(Length::Fill);

        widget::row::with_children(vec![
            sidebar,
            right.into(),
        ])
        .into()
    }
}

// ── Window handle + WebView attach ────────────────────────────────────────────

impl CosmicBrowser {
    fn try_attach_webview(&mut self) {
        if self.webview_attached { return; }

        let raw_handle = match self.core
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
                x: 220.0, y: 46.0,
                width: 1060.0, height: 720.0,
            });

        if let Ok(mut e) = self.engine.lock() {
            e.attach(
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
}
