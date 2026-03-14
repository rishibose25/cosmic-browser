use cosmic::widget::{self, button, icon, text_input};
use cosmic::iced::Alignment;
use cosmic::Element;
use crate::app::Message;

pub fn view<'a>(address: &'a str) -> Element<'a, Message> {
    let back_btn = button::icon(icon::from_name("go-previous-symbolic"))
        .on_press(Message::Back)
        .padding(8);

    let forward_btn = button::icon(icon::from_name("go-next-symbolic"))
        .on_press(Message::Forward)
        .padding(8);

    let reload_btn = button::icon(icon::from_name("view-refresh-symbolic"))
        .on_press(Message::Reload)
        .padding(8);

    let address_bar = text_input("Search or enter address…", address)
        .on_input(Message::AddressChanged)
        .on_submit(Message::Navigate(address.to_string()))
        .padding([6, 12])
        .width(cosmic::iced::Length::Fill);

    let row = widget::row::with_children(vec![
        back_btn.into(),
        forward_btn.into(),
        reload_btn.into(),
        address_bar.into(),
    ])
    .align_items(Alignment::Center)
    .spacing(4)
    .padding([6, 12]);

    widget::container(row)
        .style(cosmic::theme::Container::Background)
        .into()
}
