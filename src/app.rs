use cosmic::app::{Command, Core, Settings};
use cosmic::iced::Length;
use cosmic::widget::{self, nav_bar};
use cosmic::{Application, ApplicationExt, Element, Theme};

use crate::sidebar::SidebarTab;
use crate::toolbar::Toolbar;
use crate::browser::BrowserEngine;

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(String),
    AddressChanged(String),
    NewTab,
    CloseTab(usize),
    SelectTab(usize),
    Back,
    Forward,
    Reload,
}

pub struct CosmicBrowser {
    core: Core,
    tabs: Vec<SidebarTab>,
    active_tab: usize,
    address_input: String,
    engine: BrowserEngine,
}

impl Application for CosmicBrowser {
    type Message = Message;
    type Executor = cosmic::executor::Default;
    type Flags = ();
    const APP_ID: &'static str = "com.example.CosmicBrowser";

    fn core(&self) -> &Core { &self.core }
    fn core_mut(&mut self) -> &mut Core { &mut self.core }

    fn init(core: Core, _flags: ()) -> (Self, Command<Message>) {
        let initial_tab = SidebarTab::new("New Tab", "https://start.page");
        (
            Self {
                core,
                tabs: vec![initial_tab],
                active_tab: 0,
                address_input: String::from("https://start.page"),
                engine: BrowserEngine::new(),
            },
            Command::none(),
        )
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Navigate(url) => {
                self.address_input = url.clone();
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.url = url.clone();
                }
                self.engine.navigate(&url);
            }
            Message::AddressChanged(input) => {
                self.address_input = input;
            }
            Message::NewTab => {
                self.tabs.push(SidebarTab::new("New Tab", "https://start.page"));
                self.active_tab = self.tabs.len() - 1;
            }
            Message::CloseTab(idx) => {
                if self.tabs.len() > 1 {
                    self.tabs.remove(idx);
                    self.active_tab = self.active_tab.min(self.tabs.len() - 1);
                }
            }
            Message::SelectTab(idx) => {
                self.active_tab = idx;
                if let Some(tab) = self.tabs.get(idx) {
                    self.address_input = tab.url.clone();
                    self.engine.navigate(&tab.url);
                }
            }
            Message::Back => self.engine.back(),
            Message::Forward => self.engine.forward(),
            Message::Reload => self.engine.reload(),
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        // Left vertical tab sidebar (Zen-style)
        let sidebar = crate::sidebar::view(&self.tabs, self.active_tab);

        // Top toolbar with back/forward/reload + address bar
        let toolbar = crate::toolbar::view(&self.address_input);

        // Content area: WebView is rendered here by wry separately;
        // we render a placeholder that reserves the space
        let content = widget::container(
            widget::text("WebView renders here")
                .size(14)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(cosmic::theme::Container::Background);

        // Main layout: sidebar | (toolbar / content)
        let right_pane = widget::column::with_children(vec![
            toolbar,
            content.into(),
        ]);

        widget::row::with_children(vec![
            sidebar,
            right_pane.into(),
        ])
        .into()
    }
}

pub fn run() {
    let settings = Settings::default()
        .size((1280, 800))
        .resizable(true);
    cosmic::app::run::<CosmicBrowser>(settings, ()).unwrap();
}
