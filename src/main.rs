#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    time::Duration,
};

use iced::{
    daemon::Appearance,
    event,
    futures::{channel::mpsc::{self, Sender}, SinkExt, Stream, StreamExt},
    mouse::Button,
    stream,
    window::{self, Position, Settings},
    Color, Element, Point, Size, Subscription, Task,
};
use reqwest::Client;
use screens::Screen;
use serde_json::Value;
use tokio::time::sleep;
use util::RGB;

mod config;
mod screens;
mod style;
mod util;

fn main() {
    iced::application(KCOverlay::title, KCOverlay::update, KCOverlay::view)
        .subscription(KCOverlay::subscription)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .window(Settings {
            size: Size::new(640., 420.),
            position: Position::Specific(Point::new(0., 50.)),
            resizable: false,
            decorations: false,
            transparent: true,
            ..Default::default()
        })
        .style(|_, _| Appearance {
            background_color: Color::from_rgba8(0, 0, 0, 0.5),
            text_color: Color::WHITE,
        })
        .run_with(KCOverlay::new)
        .unwrap();
}

#[derive(Default)]
struct KCOverlay {
    screen: Screen,
    players: Vec<Player>,
    loading: bool,
    client: MineClient,
    sender: Option<mpsc::Sender<MineClient>>
}

#[derive(Debug, Clone)]
enum Message {
    None(bool),
    ChangeScreen(Screen),
    Activation(Vec<Player>),
    Log(LogReader),
    ChangeLevel(bool),
    GotEvent(event::Event),
    Close,
    ClientSelect(MineClient),
}

impl KCOverlay {
    fn new() -> (Self, Task<Message>) {
        let is_first_use = config::check_config_file();

        let config = config::get_config();
        let client = match config["client"].as_i64().unwrap_or(0) {
            0 => MineClient::Default,
            1 => MineClient::Badlion,
            2 => MineClient::Lunar,
            _ => MineClient::Default,
        };

        let screen = if is_first_use {
            Screen::Welcome
        } else {
            Screen::Main
        };

        (
            Self {
                screen,
                players: vec![],
                loading: false,
                client,
                sender: None
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        format!("KC Overlay {}", env!("CARGO_PKG_VERSION"))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::None(_) => Task::none(),

            Message::ChangeScreen(screen) => {
                self.screen = screen;
                Task::none()
            }
            Message::Activation(mut players) => {
                players.sort_by(|a, b| b.level.partial_cmp(&a.level).unwrap());
                players.truncate(16);
                self.players = players;
                self.loading = false;

                Task::perform(util::wait(Duration::from_secs(10)), Message::ChangeLevel)
            }
            Message::Log(log_reader) => match log_reader {
                LogReader::Log(message) => {
                    if message.contains("[CHAT] Jogadores") {
                        let split = message.split("):").map(|x| x.to_string());
                        let split_vector: Vec<String> = split.clone().collect();

                        let str_players: Vec<String> = split_vector[1]
                            .trim()
                            .replace(" ", "")
                            .replace("+", "")
                            .split(',')
                            .map(|x| x.to_string())
                            .collect();

                        self.players.clear();
                        self.loading = true;

                        Task::batch(vec![
                            Task::perform(get_players_info(str_players), Message::Activation),
                            window::get_latest().and_then(|x| {
                                window::change_level(x, iced::window::Level::AlwaysOnTop)
                            }),
                            window::get_latest().and_then(window::enable_mouse_passthrough),
                        ])
                    } else {
                        Task::none()
                    }
                }
                LogReader::Sender(mut sender) => {
                    let client = self.client.clone();
                    self.sender = Some(sender.clone());
                    Task::future(async move {sender.send(client).await.unwrap()}).discard()
                }
            },
            Message::ChangeLevel(_) => Task::batch(vec![
                window::get_latest()
                    .and_then(|x| window::change_level(x, iced::window::Level::AlwaysOnBottom)),
                window::get_latest().and_then(window::disable_mouse_passthrough),
            ]),

            Message::GotEvent(event) => match event {
                iced::Event::Mouse(event) => match event {
                    iced::mouse::Event::ButtonPressed(button) => {
                        if button == Button::Left {
                            window::get_latest().and_then(|x| window::drag(x))
                        } else {
                            Task::none()
                        }
                    }
                    _ => Task::none(),
                },
                _ => Task::none(),
            },
            Message::Close => window::get_latest().and_then(window::close),
            Message::ClientSelect(mine_client) => {
                self.client = mine_client.clone();

                let mut config = config::get_config();
                let client_number = match mine_client {
                    MineClient::Default => 0,
                    MineClient::Badlion => 1,
                    MineClient::Lunar => 2,
                };

                config["client"] = serde_json::json!(client_number);

                let mut config_file = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(config::get_config_file_path())
                    .unwrap();
                config_file
                    .write_all(serde_json::to_string_pretty(&config).unwrap().as_bytes())
                    .unwrap();

                match self.screen{
                    Screen::Welcome => self.screen = Screen::Main,
                    _ => ()
                }

                match &self.sender {
                    Some(sender) => {
                        Task::perform(update_client(sender.clone(), self.client.clone()), Message::None)
                    },
                    None => Task::none(),
                }
            }
        }
    }

    fn view(&self) -> Element<Message> {
        screens::get_screen(self.screen, self).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let event = event::listen().map(Message::GotEvent);
        let command_reader = Subscription::run(read_command).map(Message::Log);

        Subscription::batch(vec![event, command_reader])
    }
}

#[derive(Debug, Clone)]
enum LogReader {
    Log(String),
    Sender(mpsc::Sender<MineClient>),
}
// Read the game logs to get the player list when user types /jogando
fn read_command() -> impl Stream<Item = LogReader> {
    stream::channel(100, |mut output| async move {
        let (sender, mut receiver) = mpsc::channel(100);

        output.send(LogReader::Sender(sender)).await.unwrap();

        let client = receiver.select_next_some().await;
        let minecraft_dir = util::get_minecraft_dir();

        let logs_path = match client{
            MineClient::Default => format!("{}/logs/latest.log", minecraft_dir),
            MineClient::Badlion => format!("{}/logs/blclient/minecraft/latest.log", minecraft_dir),
            MineClient::Lunar => util::lunar_get_newer_logs_path(),
        };

        let file = File::open(&logs_path).unwrap();

        let mut reader = BufReader::new(file);
        let mut buffer = String::new();
        reader.seek(SeekFrom::End(0)).unwrap();

        loop {
            match reader.read_line(&mut buffer) {
                Ok(0) => {
                    sleep(Duration::from_millis(500)).await;
                }
                Ok(_) => {
                    let line = buffer.trim_end().to_string();
                    buffer.clear();
                    output.send(LogReader::Log(line)).await.unwrap();
                }
                Err(e) => println!("Error at reading logs: {e}"),
            }

            match receiver.try_next(){
                Ok(Some(message)) => {
                    let logs_path = match message{
                        MineClient::Default => format!("{}/logs/latest.log", minecraft_dir),
                        MineClient::Badlion => format!("{}/logs/blclient/minecraft/latest.log", minecraft_dir),
                        MineClient::Lunar => util::lunar_get_newer_logs_path(),
                    };
            
                    let file = File::open(&logs_path).unwrap();
            
                    reader = BufReader::new(file);
                    buffer = String::new();
                    reader.seek(SeekFrom::End(0)).unwrap();
                },
                Ok(None) => (),
                Err(_) => (),
            }
        }
    })
}

async fn update_client(mut sender: Sender<MineClient>, client: MineClient) -> bool{
    sender.send(client).await.unwrap();
    true
}

async fn get_players_info(str_players: Vec<String>) -> Vec<Player> {
    let mut players = vec![];
    let client = Client::new();
    const MUSH_API: &str = "https://mush.com.br/api/player/";
    for i in str_players {
        let url = format!("{}{}", MUSH_API, i);
        let request = client.get(url).send().await.unwrap().text().await.unwrap();
        println!("Getting {i} stats...");

        let json: Value = match serde_json::from_str(&request) {
            Ok(ok) => ok,
            Err(e) => {
                println!("{i}: {e}");
                continue;
            }
        };

        if !json["success"].as_bool().unwrap() {
            players.push(Player::new_nicked(i.to_string()));
            continue;
        }
        let response = json["response"].clone();

        if response["last_login"].as_i64().unwrap() - response["first_login"].as_i64().unwrap()
            < 10800000
        {
            players.push(Player::new_possible_cheater(i.to_string()));
            continue;
        }

        let username_color = response["rank_tag"]["color"].as_str().unwrap();
        let (clan, clan_color) = if response["clan"].is_object() {
            (
                Some(response["clan"]["tag"].as_str().unwrap().to_string()),
                response["clan"]["tag_color"].as_str().unwrap(),
            )
        } else {
            (None, "#ffffff")
        };

        let bedwars_stats = response["stats"]["bedwars"].clone();
        if bedwars_stats.is_object() {
            let level = bedwars_stats["level"].as_i64().unwrap_or(0);
            let level_symbol = bedwars_stats["level_badge"]["symbol"]
                .as_str()
                .unwrap()
                .to_string();
            let level_color = bedwars_stats["level_badge"]["hex_color"]
                .as_str()
                .unwrap()
                .to_string();

            let winstreak = bedwars_stats["winstreak"].as_i64().unwrap_or(0);

            let winrate = bedwars_stats["wins"].as_i64().unwrap_or(0) as f32
                / bedwars_stats["losses"].as_i64().unwrap_or(0) as f32;
            let final_kill_final_death_ratio = bedwars_stats["final_kills"].as_i64().unwrap_or(0)
                as f32
                / bedwars_stats["final_deaths"].as_i64().unwrap_or(0) as f32;

            players.push(Player::new(
                i,
                RGB::from_hex(username_color),
                level as i32,
                level_symbol,
                winstreak as i32,
                clan,
                RGB::from_hex(clan_color),
                winrate,
                final_kill_final_death_ratio,
                RGB::from_hex(&level_color),
            ));
        }
        sleep(Duration::from_millis(60)).await;
    }
    players
}

#[derive(Debug, Clone)]
struct Player {
    username: String,
    username_color: RGB,
    level: i32,
    level_symbol: String,
    winstreak: i32,
    clan: Option<String>,
    clan_color: RGB,
    is_nicked: bool,
    is_possible_cheater: bool,
    winrate: f32,
    final_kill_final_death_ratio: f32,
    level_color: RGB,
}

impl Player {
    fn new(
        username: String,
        username_color: RGB,
        level: i32,
        level_symbol: String,
        winstreak: i32,
        clan: Option<String>,
        clan_color: RGB,
        winrate: f32,
        final_kill_final_death_ratio: f32,
        level_color: RGB,
    ) -> Self {
        Player {
            username,
            username_color,
            level,
            level_symbol,
            winstreak,
            clan,
            clan_color,
            is_nicked: false,
            is_possible_cheater: false,
            winrate,
            final_kill_final_death_ratio,
            level_color,
        }
    }

    fn new_nicked(username: String) -> Self {
        Player {
            username,
            username_color: RGB::new(0, 255, 255),
            level: 0,
            level_symbol: "?".to_string(),
            winstreak: 0,
            clan: None,
            clan_color: RGB::new(0, 0, 0),
            is_nicked: true,
            is_possible_cheater: false,
            winrate: 0.,
            final_kill_final_death_ratio: 0.,
            level_color: RGB::new(0, 255, 255),
        }
    }

    fn new_possible_cheater(username: String) -> Self {
        Player {
            username,
            username_color: RGB::new(255, 0, 0),
            level: 0,
            level_symbol: "?".to_string(),
            winstreak: 0,
            clan: None,
            clan_color: RGB::new(0, 0, 0),
            is_nicked: false,
            is_possible_cheater: true,
            winrate: 0.,
            final_kill_final_death_ratio: 0.,
            level_color: RGB::new(0, 255, 255),
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
enum MineClient {
    #[default]
    Default,
    Badlion,
    Lunar,
}

impl ToString for MineClient{
    fn to_string(&self) -> String {
        match self{
            MineClient::Default => "Vanilla".to_string(),
            MineClient::Badlion => "Badlion".to_string(),
            MineClient::Lunar => "Lunar".to_string(),
        }
    }
}