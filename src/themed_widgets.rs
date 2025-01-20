use std::borrow::Borrow;

use iced::{border::Radius, widget::{PickList, TextInput}, Background, Border, Color, Renderer, Shadow, Theme};

use crate::Message;

enum Colors{
    WidgetBackground,
    WidgedBackgroundHighlight,
    ButtonColor,
    SecondaryButtonColor,
    TextColor,
    GrayTextColor
}
impl Colors{
    fn get(&self) -> Color{
        match self{
            Colors::WidgetBackground => Color::from_rgb8(54, 58, 79),
            Colors::WidgedBackgroundHighlight => Color::from_rgb8(73, 77, 100),
            Colors::ButtonColor => Color::from_rgb8(30, 102, 245),
            Colors::SecondaryButtonColor => Color::from_rgb8(64, 160, 43),
            Colors::TextColor => Color::from_rgb8(255, 255, 255),
            Colors::GrayTextColor => Color::from_rgb8(200, 200, 200)
        }
    }
}
pub fn button<'a>(
    content: impl Into<iced::Element<'a, Message, Theme, Renderer>>,
) -> iced::widget::Button<'a, Message, Theme, Renderer> {
    iced::widget::button(content).style(move |_: &Theme, _| iced::widget::button::Style {
        background: Some(Background::Color(Color::from_rgb8(30, 102, 245))),
        text_color: Color::from_rgb8(255, 255, 255),
        border: Border {
            color: Color::from_rgb8(30, 102, 245),
            width: 0.,
            radius: Radius::new(10),
        },
        shadow: Shadow::default(),
    })
}

pub fn secondary_button<'a>(
    content: impl Into<iced::Element<'a, Message, Theme, Renderer>>,
) -> iced::widget::Button<'a, Message, Theme, Renderer> {
    iced::widget::button(content).style(move |_: &Theme, _| iced::widget::button::Style {
        background: Some(Background::Color(Colors::SecondaryButtonColor.get())),
        text_color: Color::from_rgb8(255, 255, 255),
        border: Border {
            color: Color::from_rgb8(64, 160, 43),
            width: 0.,
            radius: Radius::new(10),
        },
        shadow: Shadow::default(),
    })
}

pub fn pick_list<'a, T, L, V, Message>(
    options: L,
    selected: Option<V>,
    on_selected: impl Fn(T) -> Message + 'a,
) -> PickList<'a, T, L, V, Message, Theme, Renderer>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
    Message: Clone,
{
    iced::widget::pick_list(options, selected, on_selected)
        .style(move |_: &Theme, _| iced::widget::pick_list::Style {
            text_color: Color::from_rgb8(255, 255, 255),
            placeholder_color: Color::from_rgb8(255, 255, 255),
            handle_color: Color::from_rgb8(30, 102, 245),
            background: Background::Color(Colors::WidgetBackground.get()),
            border: Border {
                color: Colors::WidgetBackground.get(),
                width: 0.,
                radius: Radius::new(10),
            },
        })
        .menu_style(|_| iced::overlay::menu::Style {
            background: Background::Color(Colors::WidgetBackground.get()),
            border: Border {
                color: Colors::WidgetBackground.get(),
                width: 0.,
                radius: Radius::new(10),
            },
            text_color: Color::from_rgb8(255, 255, 255),
            selected_text_color: Color::from_rgb8(255, 255, 255),
            selected_background: Background::Color(Colors::WidgedBackgroundHighlight.get()),
        })
}

pub fn text_input<'a, Message>(
    placeholder: &str,
    value: &str,
) -> TextInput<'a, Message, Theme, Renderer>
where
    Message: Clone,
{
    iced::widget::text_input(placeholder, value).style(|_, _| iced::widget::text_input::Style{
        background: Background::Color(Colors::WidgetBackground.get()),
        border: Border {
            color: Colors::WidgetBackground.get(),
            width: 0.,
            radius: Radius::new(10),
        },
        icon: Colors::GrayTextColor.get(),
        placeholder: Colors::GrayTextColor.get(),
        value: Colors::TextColor.get(),
        selection: Colors::ButtonColor.get(),
    })
}
