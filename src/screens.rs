use iced::{
    border::Radius,
    theme,
    widget::{
        button::{Catalog, Style},
        column, container, pick_list, row, text, Column,
    },
    Background, Border, Color, Element, Renderer, Shadow, Theme,
};

use crate::{Message, MineClient};

#[derive(Clone, Copy, Default, Debug)]
pub enum Screen {
    #[default]
    Main,
    Settings,
    Welcome,
}

pub fn get_screen(
    screen: Screen,
    app: &super::KCOverlay,
) -> Column<'static, super::Message, theme::Theme, Renderer> {
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

            let mut username_column = Column::new().width(280);
            let mut winstreak_column = Column::new();
            let mut winrate_column = Column::new();
            let mut fkfdr_column = Column::new();

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
                let winstreak_widget = text(format!("{} winstreak", player.winstreak));
                let winrate_widget = text(format!("{:.2} winrate", player.winrate));
                let fkfdr = text(format!("{:.2} fkfdr", player.final_kill_final_death_ratio));

                let mut username_row = row![level_widget, username_widget, clan_widget];
                if player.is_nicked {
                    username_row =
                        username_row.push(text("</nick]>").color(Color::from_rgb8(0, 0, 128)))
                } else if player.is_possible_cheater {
                    username_row = username_row
                        .push(text("<possível CHEATER>").color(Color::from_rgb8(255, 0, 0)).size(12))
                }
                username_column = username_column.push(username_row);
                winstreak_column = winstreak_column.push(winstreak_widget);
                winrate_column = winrate_column.push(winrate_widget);
                fkfdr_column = fkfdr_column.push(fkfdr);
            }
            let column_row = row![
                username_column,
                winstreak_column,
                winrate_column,
                fkfdr_column
            ]
            .spacing(5);
            let container = container(column_row).height(355);

            let settings =
                button("Configurações").on_press(Message::ChangeScreen(Screen::Settings));
            let close = button("Sair").on_press(Message::Close);
            let minimize = button("Minimizar").on_press(Message::Minimize);

            let bottom_row = row![settings, minimize, close].spacing(20);

            column![bar, container, bottom_row].padding(10)
        }
        Screen::Settings => {
            use super::MineClient;
            let clients = vec![MineClient::Default, MineClient::Badlion, MineClient::Lunar];
            let client_select = pick_list(clients, Some(app.client.clone()), Message::ClientSelect);
            let client_row = row![text("Client:"), client_select].spacing(10);
            let go_back = button("Voltar").on_press(Message::ChangeScreen(Screen::Main));

            column![client_row, go_back].padding(10).spacing(10)
        }
        Screen::Welcome => {
            let welcome_text = text("Muito obrigado por usar a overlay! Selecione o client que você usa para proseguir.");

            let default_client =
                button("Vanilla / outro").on_press(Message::ClientSelect(MineClient::Default));
            let default_client_text = text("Para Vanilla e CMclient.");
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
    }
}

fn button<'a>(content: &str) -> iced::widget::Button<'_, Message, Theme, Renderer> {
    iced::widget::button(content).style(move |_: &Theme, _| Style {
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
