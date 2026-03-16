use cosmic::{
    widget::{self, button, icon, settings, toggler, text},
    Element,
};
use crate::settings::{
    AppearanceSettings, BrowserSettings, GeneralSettings,
    PrivacySettings, SearchEngine, SearchSettings, SidebarPosition, Theme,
};

// ── Messages ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    // General
    HomepageChanged(String),
    NewTabUrlChanged(String),
    RestoreSessionToggled(bool),
    // Privacy
    JavascriptToggled(bool),
    CookiesToggled(bool),
    BlockThirdPartyToggled(bool),
    DoNotTrackToggled(bool),
    ClearOnCloseToggled(bool),
    // Appearance
    ThemeSelected(Theme),
    SidebarPositionSelected(SidebarPosition),
    StatusBarToggled(bool),
    // Search
    SearchEngineSelected(SearchEngine),
    CustomSearchUrlChanged(String),
    SuggestToggled(bool),
    // Actions
    Save,
    Close,
}

// ── View ──────────────────────────────────────────────────────────────────────

pub fn view(s: &BrowserSettings) -> Element<SettingsMessage> {
    let content = widget::column::with_children(vec![
        section_general(&s.general),
        section_search(&s.search),
        section_appearance(&s.appearance),
        section_privacy(&s.privacy),
        save_row(),
    ])
    .spacing(24)
    .padding([24, 32]);

    widget::scrollable(content)
        .height(cosmic::iced::Length::Fill)
        .into()
}

// ── General section ───────────────────────────────────────────────────────────

fn section_general(g: &GeneralSettings) -> Element<'_, SettingsMessage> {
    settings::section()
        .title("General")
        .add(settings::item(
            "Homepage",
            widget::text_input("https://…", &g.homepage)
                .on_input(SettingsMessage::HomepageChanged),
        ))
        .add(settings::item(
            "New tab page",
            widget::text_input("https://…", &g.new_tab_url)
                .on_input(SettingsMessage::NewTabUrlChanged),
        ))
        .add(settings::item(
            "Restore session on launch",
            toggler(
                None,
                g.restore_session,
                SettingsMessage::RestoreSessionToggled,
            ),
        ))
        .into()
}

// ── Search section ────────────────────────────────────────────────────────────

fn section_search(s: &SearchSettings) -> Element<'_, SettingsMessage> {
    let engines = [
        SearchEngine::Brave,
        SearchEngine::DuckDuckGo,
        SearchEngine::Google,
        SearchEngine::Startpage,
        SearchEngine::Custom,
    ];

    let engine_buttons = engines.iter().fold(
        widget::row::with_capacity(5).spacing(8),
        |row, engine| {
            let selected = *engine == s.engine;
            let btn = if selected {
                button::text(engine.label())
                    .style(cosmic::theme::Button::Suggested)
                    .on_press(SettingsMessage::SearchEngineSelected(engine.clone()))
            } else {
                button::text(engine.label())
                    .style(cosmic::theme::Button::Standard)
                    .on_press(SettingsMessage::SearchEngineSelected(engine.clone()))
            };
            row.push(btn)
        },
    );

    let mut section = settings::section()
        .title("Search")
        .add(settings::item("Search engine", engine_buttons))
        .add(settings::item(
            "Search suggestions",
            toggler(None, s.suggest, SettingsMessage::SuggestToggled),
        ));

    if s.engine == SearchEngine::Custom {
        section = section.add(settings::item(
            "Custom search URL",
            widget::text_input(
                "https://example.com/search?q=%s",
                s.custom_url.as_deref().unwrap_or(""),
            )
            .on_input(SettingsMessage::CustomSearchUrlChanged),
        ));
    }

    section.into()
}

// ── Appearance section ────────────────────────────────────────────────────────

fn section_appearance(a: &AppearanceSettings) -> Element<'_, SettingsMessage> {
    let themes = [Theme::System, Theme::Light, Theme::Dark];
    let theme_labels = ["Follow system", "Light", "Dark"];

    let theme_buttons = themes.iter().zip(theme_labels.iter()).fold(
        widget::row::with_capacity(3).spacing(8),
        |row, (theme, label)| {
            let selected = *theme == a.theme;
            let btn = if selected {
                button::text(*label)
                    .style(cosmic::theme::Button::Suggested)
                    .on_press(SettingsMessage::ThemeSelected(theme.clone()))
            } else {
                button::text(*label)
                    .style(cosmic::theme::Button::Standard)
                    .on_press(SettingsMessage::ThemeSelected(theme.clone()))
            };
            row.push(btn)
        },
    );

    let sidebar_buttons = widget::row::with_children(vec![
        if a.sidebar_position == SidebarPosition::Left {
            button::text("Left")
                .style(cosmic::theme::Button::Suggested)
                .on_press(SettingsMessage::SidebarPositionSelected(SidebarPosition::Left))
                .into()
        } else {
            button::text("Left")
                .style(cosmic::theme::Button::Standard)
                .on_press(SettingsMessage::SidebarPositionSelected(SidebarPosition::Left))
                .into()
        },
        if a.sidebar_position == SidebarPosition::Right {
            button::text("Right")
                .style(cosmic::theme::Button::Suggested)
                .on_press(SettingsMessage::SidebarPositionSelected(SidebarPosition::Right))
                .into()
        } else {
            button::text("Right")
                .style(cosmic::theme::Button::Standard)
                .on_press(SettingsMessage::SidebarPositionSelected(SidebarPosition::Right))
                .into()
        },
    ])
    .spacing(8);

    settings::section()
        .title("Appearance")
        .add(settings::item("Theme", theme_buttons))
        .add(settings::item("Sidebar position", sidebar_buttons))
        .add(settings::item(
            "Show status bar",
            toggler(None, a.show_status_bar, SettingsMessage::StatusBarToggled),
        ))
        .into()
}

// ── Privacy section ───────────────────────────────────────────────────────────

fn section_privacy(p: &PrivacySettings) -> Element<'_, SettingsMessage> {
    settings::section()
        .title("Privacy & security")
        .add(settings::item(
            "Enable JavaScript",
            toggler(None, p.javascript_enabled, SettingsMessage::JavascriptToggled),
        ))
        .add(settings::item(
            "Enable cookies",
            toggler(None, p.cookies_enabled, SettingsMessage::CookiesToggled),
        ))
        .add(settings::item(
            "Block third-party cookies",
            toggler(
                None,
                p.block_third_party_cookies,
                SettingsMessage::BlockThirdPartyToggled,
            ),
        ))
        .add(settings::item(
            "Send Do Not Track",
            toggler(None, p.do_not_track, SettingsMessage::DoNotTrackToggled),
        ))
        .add(settings::item(
            "Clear data on close",
            toggler(None, p.clear_on_close, SettingsMessage::ClearOnCloseToggled),
        ))
        .into()
}

// ── Save row ──────────────────────────────────────────────────────────────────

fn save_row<'a>() -> Element<'a, SettingsMessage> {
    widget::row::with_children(vec![
        widget::horizontal_space(cosmic::iced::Length::Fill).into(),
        button::text("Cancel")
            .style(cosmic::theme::Button::Standard)
            .on_press(SettingsMessage::Close)
            .into(),
        button::text("Save")
            .style(cosmic::theme::Button::Suggested)
            .on_press(SettingsMessage::Save)
            .into(),
    ])
    .spacing(8)
    .into()
}
