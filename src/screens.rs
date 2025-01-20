use std::borrow::Borrow;

use iced::{
    border::Radius,
    theme,
    widget::{column, container, row, text, Column, PickList},
    Background, Border, Color, Renderer, Shadow, Theme,
};

use crate::{Message, MineClient};

#[derive(Clone, Copy, Default, Debug)]
pub enum Screen {
    #[default]
    Main,
    Settings,
    Welcome,
    Info
}

pub fn get_screen(
    screen: Screen,
    app: &super::KCOverlay,
) -> Column<'static, super::Message, theme::Theme, Renderer> {

    const COLUMN_HEIGHT: u16 = 375;

    match screen {
        Screen::Main => {
            let screen_title_text = if app.players.is_empty() {
                String::from(
                    "Digite o comando /jogando no chat do mush para ver os stats dos jogadores",
                )
            } else if app.loading {
                String::from("Carregando jogadores...")
            } else {
                format!("Top {} jogadores da sala", app.players.len())
            };

            let screen_title_widget = text(screen_title_text);

            let bar = row![screen_title_widget];

            let mut username_column = Column::new().width(300);
            let mut winstreak_column = Column::new();
            let mut winrate_column = Column::new();
            let mut fkdr_column = Column::new();

            let players = app.players.clone();
            for player in players {
                let clan = if let Some(value) = &player.clan {
                    format!("[{}]", value)
                } else {
                    String::new()
                };

                let level_widget =
                    text(format!("[{}] ", player.level)).color(player.level_color.to_color());
                let username_widget = text(player.username).color(player.username_color.to_color());
                let clan_widget = text(clan).color(player.clan_color.to_color());
                let winstreak_widget = text(format!("{} ws", player.winstreak));
                let winrate_widget = text(format!("{:.2} winrate", player.winrate));
                let fkdr = text(format!("{:.2} fkdr", player.final_kill_final_death_ratio));

                let mut username_row = row![level_widget, username_widget, clan_widget];
                if player.is_nicked {
                    username_row =
                        username_row.push(text("</nick>").color(Color::from_rgb8(255, 255, 0)))
                } else if player.is_possible_cheater {
                    username_row = username_row.push(
                        text("<possível CHEATER>")
                            .color(Color::from_rgb8(255, 0, 0))
                            .size(12),
                    )
                }
                username_column = username_column.push(username_row);
                winstreak_column = winstreak_column.push(winstreak_widget);
                winrate_column = winrate_column.push(winrate_widget);
                fkdr_column = fkdr_column.push(fkdr);
            }
            let column_row = row![
                username_column,
                winstreak_column,
                winrate_column,
                fkdr_column
            ]
            .spacing(10);
            let container = container(column_row);

            let settings =
                button("Configurações").on_press(Message::ChangeScreen(Screen::Settings));
            let close = button("Sair").on_press(Message::Close);
            let minimize = button("Minimizar").on_press(Message::Minimize);
            let info = button("Sobre").on_press(Message::ChangeScreen(Screen::Info));

            let mut bottom_row = row![settings, info, minimize, close].spacing(20);
            if app.update.available{
                let update_button = secondary_button("Atualizar").on_press(Message::Update);
                bottom_row = bottom_row.push(update_button);
            }

            let main_column = column![bar, container].spacing(10).height(COLUMN_HEIGHT);

            column![main_column, bottom_row].padding(10).spacing(10)
        }
        Screen::Settings => {
            use super::MineClient;
            let clients = vec![MineClient::Default, MineClient::Badlion, MineClient::Lunar, MineClient::LegacyLauncher];
            let client_select = pick_list(clients, Some(app.client.clone()), Message::ClientSelect);
            let client_row = row![text("Client:"), client_select].spacing(10);
            let go_back = button("Voltar").on_press(Message::ChangeScreen(Screen::Main));

            let main_column = column![client_row].spacing(10).height(COLUMN_HEIGHT);

            column![main_column, go_back].padding(10).spacing(10)
        }
        Screen::Welcome => {
            let welcome_text = text("Muito obrigado por usar a overlay! Selecione o client que você usa para proseguir.");

            let default_client =
                button("Geral / outro").on_press(Message::ClientSelect(MineClient::Default));
            let default_client_text = text("Vanilla, CMClient, Forge, etc");
            let dafault_client_row = row![default_client, default_client_text].spacing(10);

            let badlion = button("Badlion").on_press(Message::ClientSelect(MineClient::Badlion));

            let lunar = button("Lunar").on_press(Message::ClientSelect(MineClient::Lunar));

            let legacy_launcher = button("Legacy Launcher")
                .on_press(Message::ClientSelect(MineClient::LegacyLauncher));

            column![
                welcome_text,
                dafault_client_row,
                badlion,
                lunar,
                legacy_launcher
            ]
            .spacing(10)
            .padding(10)
        }
        Screen::Info => {
            let thanks_text = text("Muito obrigado por usar o KC Overlay! Considere virar membro do Discord para saber das novidades");
            let discord_button = button("Entrar no discord").on_press(Message::OpenLink(String::from("https://discord.gg/SqT7YHSGzJ")));

            let creditos = text(format!("KC Overlay {} - Criado por Jafkc2",  env!("CARGO_PKG_VERSION")));
            let github = button("Acessar Github").on_press(Message::OpenLink(String::from("https://github.com/jafkc2/KC-Overlay")));

            let go_back = button("Voltar").on_press(Message::ChangeScreen(Screen::Main));

            let discord_column = column![thanks_text, discord_button].spacing(10).height(125);
            let credits_column = column![creditos, github].spacing(10);

            let main_column = column![discord_column, credits_column].height(COLUMN_HEIGHT);


            column![main_column, go_back].spacing(10).padding(10)
        },
    }
}

fn button<'a>(content: impl Into<iced::Element<'a, Message, Theme, Renderer>>) -> iced::widget::Button<'a, Message, Theme, Renderer> {
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

fn secondary_button<'a>(content: impl Into<iced::Element<'a, Message, Theme, Renderer>>) -> iced::widget::Button<'a, Message, Theme, Renderer> {
    iced::widget::button(content).style(move |_: &Theme, _| iced::widget::button::Style {
        background: Some(Background::Color(Color::from_rgb8(64, 160, 43))),
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
            background: Background::Color(Color::from_rgb8(54, 58, 79)),
            border: Border {
                color: Color::from_rgb8(54, 58, 79),
                width: 0.,
                radius: Radius::new(10),
            },
        })
        .menu_style(|_| iced::overlay::menu::Style {
            background: Background::Color(Color::from_rgb8(54, 58, 79)),
            border: Border {
                color: Color::from_rgb8(54, 58, 79),
                width: 0.,
                radius: Radius::new(10),
            },
            text_color: Color::from_rgb8(255, 255, 255),
            selected_text_color: Color::from_rgb8(255, 255, 255),
            selected_background: Background::Color(Color::from_rgb8(73, 77, 100)),
        })
}