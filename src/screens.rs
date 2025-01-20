use iced::{
    theme,
    widget::{column, container, row, text, Column},
    Color, Renderer
};

use crate::{themed_widgets::{button, pick_list, secondary_button, text_input}, Message, MineClient};

#[derive(Clone, Copy, Default, Debug)]
pub enum Screen {
    #[default]
    Main,
    Settings,
    Welcome,
    Info,
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
            if app.update.available {
                let update_button = secondary_button("Atualizar").on_press(Message::Update);
                bottom_row = bottom_row.push(update_button);
            }

            let main_column = column![bar, container].spacing(10).height(COLUMN_HEIGHT);

            column![main_column, bottom_row].padding(10).spacing(10)
        }
        Screen::Settings => {
            use super::MineClient;
            let clients = vec![
                MineClient::Default,
                MineClient::Badlion,
                MineClient::Lunar,
                MineClient::LegacyLauncher,
                MineClient::Custom(" ".to_string()),
            ];
            let client_select = pick_list(clients, Some(app.client.clone()), Message::ClientSelect);
            let client_row = row![text("Client:"), client_select].spacing(10);

            let go_back = button("Voltar").on_press(Message::ChangeScreen(Screen::Main));

            let mut main_column = column![client_row].spacing(10).height(COLUMN_HEIGHT);

            match &app.client {
                MineClient::Custom(path) => {
                    let custom_client_text = text("Último log do client (ex: logs/latest.log):");
                    let custom_client_path =
                        text_input("Insira o último arquivo de log", &path)
                            .on_input(Message::CustomClientPathModified);
                    let custom_client_search = button("Procurar").on_press(Message::SearchExplorer);

                    let custom_client_row =
                        row![custom_client_path, custom_client_search].spacing(10);

                    main_column = main_column
                        .push(column![custom_client_text, custom_client_row])
                        .spacing(10)
                }
                _ => (),
            }

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
            let discord_button = button("Entrar no discord").on_press(Message::OpenLink(
                String::from("https://discord.gg/SqT7YHSGzJ"),
            ));

            let creditos = text(format!(
                "KC Overlay {} - Criado por Jafkc2",
                env!("CARGO_PKG_VERSION")
            ));
            let github = button("Acessar Github").on_press(Message::OpenLink(String::from(
                "https://github.com/jafkc2/KC-Overlay",
            )));

            let go_back = button("Voltar").on_press(Message::ChangeScreen(Screen::Main));

            let discord_column = column![thanks_text, discord_button].spacing(10).height(125);
            let credits_column = column![creditos, github].spacing(10);

            let main_column = column![discord_column, credits_column].height(COLUMN_HEIGHT);

            column![main_column, go_back].spacing(10).padding(10)
        }
    }
}

