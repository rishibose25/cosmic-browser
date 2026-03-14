use cosmic::widget::{self, button, icon, scrollable, text};
use cosmic::iced::Length;
use cosmic::Element;
use crate::app::Message;

#[derive(Debug, Clone)]
pub struct SidebarTab {
    pub title: String,
    pub url: String,
    pub favicon: Option<String>,
}

impl SidebarTab {
    pub fn new(title: &str, url: &str) -> Self {
        Self {
            title: title.to_string(),
            url: url.to_string(),
            favicon: None,
        }
    }
}

pub fn view<'a>(tabs: &'a [SidebarTab], active: usize) -> Element<'a, Message> {
    let mut tab_list = widget::column::with_capacity(tabs.len() + 1)
        .spacing(4)
        .padding([8, 6]);

    for (i, tab) in tabs.iter().enumerate() {
        let is_active = i == active;

        let label = text(&tab.title).size(13);

        let close_btn = button::icon(icon::from_name("window-close-symbolic"))
            .on_press(Message::CloseTab(i))
            .padding(2);

        let tab_row = widget::row::with_children(vec![
            label.into(),
            widget::horizontal_space(Length::Fill).into(),
            close_btn.into(),
        ])
        .align_items(cosmic::iced::Alignment::Center)
        .padding([6, 10])
        .spacing(4);

        let tab_btn = if is_active {
            button::custom(tab_row)
                .style(cosmic::theme::Button::Suggested)
                .on_press(Message::SelectTab(i))
        } else {
            button::custom(tab_row)
                .style(cosmic::theme::Button::Text)
                .on_press(Message::SelectTab(i))
        };

        tab_list = tab_list.push(tab_btn);
    }

    // New tab button at the bottom of the sidebar
    let new_tab_btn = button::icon(icon::from_name("list-add-symbolic"))
        .on_press(Message::NewTab)
        .padding(8);

    let sidebar_inner = widget::column::with_children(vec![
        scrollable(tab_list).height(Length::Fill).into(),
        new_tab_btn.into(),
    ])
    .height(Length::Fill)
    .padding([8, 0]);

    widget::container(sidebar_inner)
        .width(220)
        .height(Length::Fill)
        .style(cosmic::theme::Container::Secondary)
        .into()
}
