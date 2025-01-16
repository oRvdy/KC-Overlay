use iced::{gradient::Linear, Background, Border, Color, Gradient, Shadow};

pub fn container() -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: todo!(),
        background: todo!(),
        border: todo!(),
        shadow: todo!(),
    }
}
pub fn button() -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(Background::Gradient(Gradient::Linear(Linear::new(12)))),
        text_color: get_color(AppColors::Text),
        border: Border::default(),
        shadow: Shadow::default(),
    }
}

pub fn text(color: Color) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(color) }
}

enum AppColors {
    Background,
    Surface,
    Text,
    Button,
}

fn get_color(app_color: AppColors) -> Color {
    match app_color {
        AppColors::Background => Color::from_rgb8(36, 39, 58),
        AppColors::Surface => Color::from_rgb8(54, 58, 79),
        AppColors::Text => Color::from_rgb8(202, 211, 245),
        AppColors::Button => Color::from_rgb8(138, 173, 244),
    }
}
