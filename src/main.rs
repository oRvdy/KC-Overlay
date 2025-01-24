#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    env,
    fmt::Display,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::Path,
    time::Duration,
};

use iced::{
    event,
    futures::{
        channel::mpsc::{self, Sender},
        executor::block_on,
        SinkExt, Stream, StreamExt,
    },
    mouse::Button,
    stream,
    theme::Style,
    time,
    window::{self, Position, Settings},
    Color, Element, Font, Point, Size, Subscription, Task,
};
use reqwest::Client;
use screens::Screen;
use serde_json::Value;
use tokio::time::sleep;
use util::Rgb;

mod config;
mod screens;
mod themed_widgets;
mod update;
mod util;

fn main() {
    // Isso é o processo final do update. Remove o executável antigo, caso exista.
    let old_exec = env::current_exe().unwrap().with_extension("old");
    if Path::new(&old_exec).exists() {
        match fs::remove_file(old_exec) {
            Ok(ok) => ok,
            Err(e) => println!("Failed to delete old executable: {e}"),
        }
    }

    let icon = include_bytes!("../assets/icon.png");

    // Executa a lógica do programa.
    iced::application(KCOverlay::title, KCOverlay::update, KCOverlay::view)
        .subscription(KCOverlay::subscription)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .window(Settings {
            size: Size::new(640., 460.),
            position: Position::Specific(Point::new(0., 50.)),
            resizable: false,
            decorations: false,
            transparent: true,
            icon: Some(window::icon::from_file_data(icon, None).unwrap()),
            ..Default::default()
        })
        .style(|_, _| Style {
            background_color: Color::from_rgba8(24, 25, 33, 0.75),
            text_color: Color::WHITE,
        })
        .font(KCOverlay::FONT)
        .font(KCOverlay::SYMBOL_FONT)
        .default_font(Font::with_name("Manrope"))
        .scale_factor(KCOverlay::scale_factor)
        .run_with(KCOverlay::new)
        .unwrap();
}

// Estrutura do programa, aqui estão salvas todas as variáveis necessárias.
#[derive(Default)]
struct KCOverlay {
    screen: Screen,
    players: Vec<Player>,
    loading: bool,
    client: MineClient,
    logs_sender: Option<mpsc::Sender<MineClient>>,
    player_getter_sender: Option<mpsc::Sender<()>>,
    update: Update,
    never_minimize: bool,
    seconds_to_minimize: u64
}

// Mensagens enviadas para o programa saber quando atualizar variáveis, executar funções, e etc.
#[derive(Debug, Clone)]
enum Message {
    ChangeScreen(Screen),
    Log(LogReader),
    ChangeLevel,
    GotEvent(event::Event),
    Close,
    ClientSelect(MineClient),
    ClientUpdate,
    Minimize,
    PlayerSender(PlayerSender),
    CheckedUpdates(Result<String, String>),
    OpenLink(String),
    Update,
    UpdateResult(Result<(), String>),
    CustomClientPathModified(String),
    SearchExplorer,
    ChangeNeverMinimize(bool),
    ChangeSecondsToMinimize(f64)
}

// Lógica principal do programa.
impl KCOverlay {
    // Fontes :D
    const FONT: &'static [u8] = include_bytes!("../fonts/Manrope-Regular.ttf");
    const SYMBOL_FONT: &'static [u8] = include_bytes!("../fonts/NotoSansSymbols2-Regular.ttf");

    // Função executada após o início da lógica. Ela coleta os dados do arquivo de configuração.
    fn new() -> (Self, Task<Message>) {
        let is_first_use = config::check_config_file();

        let config = config::get_config();
        let custom_client_path = config["custom_client_path"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let client = match config["client"].as_i64().unwrap_or(0) {
            0 => MineClient::Default,
            1 => MineClient::Badlion,
            2 => MineClient::Lunar,
            3 => MineClient::LegacyLauncher,
            4 => MineClient::Custom(custom_client_path),
            5 => MineClient::Silent,
            _ => MineClient::Default,
        };
        let never_minimize = config["never_minimize"].as_bool().unwrap_or(false);
        let seconds_to_minimize = config["seconds_to_minimize"].as_u64().unwrap_or(10);

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
                logs_sender: None,
                player_getter_sender: None,
                update: Update::empty(),
                never_minimize,
                seconds_to_minimize
            },
            Task::batch(vec![Task::perform(
                update::check_updates(),
                Message::CheckedUpdates,
            )]),
        )
    }

    fn title(&self) -> String {
        format!("KC Overlay {}", env!("CARGO_PKG_VERSION"))
    }

    // Qualquer mensagem passará por esta função, executando sua respectiva ação.
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ChangeScreen(screen) => {
                self.screen = screen;
                Task::none()
            }

            Message::Log(log_reader) => match log_reader {
                LogReader::Log(message) => {
                    // Checa se a mensagem possui a lista de jogadores de quando o jogador digita "/jogando".
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
                            Task::run(get_players(str_players), |player_sender: PlayerSender| {
                                Message::PlayerSender(player_sender)
                            }),
                            window::get_latest().and_then(|x| {
                                window::set_level(x, iced::window::Level::AlwaysOnTop)
                            }),
                            window::get_latest().and_then(|x| window::minimize(x, false)),
                        ])
                    } else {
                        Task::none()
                    }
                }
                LogReader::Sender(mut sender) => {
                    let client = self.client.clone();
                    self.logs_sender = Some(sender.clone());
                    Task::future(async move { sender.send(client).await.unwrap() }).discard()
                }
            },
            // Minimiza a janela
            Message::ChangeLevel => {
                if self.loading {
                    Task::none()
                } else {
                    if self.never_minimize {
                        Task::none()
                    } else {
                        Task::batch(vec![
                            window::get_latest().and_then(|x| window::minimize(x, true))
                        ])
                    }
                }
            }
            // Arrasta a janela quando o botão do mouse está segurado.
            Message::GotEvent(event) => match event {
                iced::Event::Mouse(iced::mouse::Event::ButtonPressed(Button::Left)) => {
                    window::get_latest().and_then(window::drag)
                }
                _ => Task::none(),
            },
            Message::Close => window::get_latest().and_then(window::close),
            // Seleciona o Client e salva no arquivo de configuração.
            Message::ClientSelect(mine_client) => {
                self.client = mine_client.clone();

                let mut config = config::get_config();
                let client_number = match mine_client {
                    MineClient::Default => 0,
                    MineClient::Badlion => 1,
                    MineClient::Lunar => 2,
                    MineClient::LegacyLauncher => 3,
                    MineClient::Custom(path) => {
                        if !path.eq(" ") {
                            config["custom_client_path"] = serde_json::json!(path);
                        } else {
                            let custom_client_path =
                                config["custom_client_path"].as_str().unwrap().to_string();
                            self.client = MineClient::Custom(custom_client_path)
                        }
                        4
                    }
                    MineClient::Silent => 5,
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

                if let Screen::Welcome = self.screen {
                    self.screen = Screen::Main
                }

                match &self.logs_sender {
                    Some(sender) => {
                        Task::future(update_client(sender.clone(), self.client.clone())).discard()
                    }
                    None => Task::none(),
                }
            }
            Message::Minimize => window::get_latest().and_then(|x| window::minimize(x, true)),
            // Avisa ao código responsável por ler os logs que o client foi atualizado e ele precisa ler os logs de outro lugar.
            Message::ClientUpdate => match &self.logs_sender {
                Some(sender) => {
                    Task::future(update_client(sender.clone(), self.client.clone())).discard()
                }
                None => Task::none(),
            },
            // Gerencia o output do código responsável por ler os logs.
            Message::PlayerSender(player_sender) => match player_sender {
                PlayerSender::Player(player) => {
                    self.players.push(player);
                    self.players
                        .sort_by(|a, b| b.level.partial_cmp(&a.level).unwrap());
                    self.players.truncate(16);
                    Task::none()
                }
                PlayerSender::Done => {
                    self.loading = false;
                    self.player_getter_sender = None;
                    Task::perform(util::wait(Duration::from_secs(self.seconds_to_minimize)), |_| {
                        Message::ChangeLevel
                    })
                }
                PlayerSender::Sender(new_sender) => {
                    match self.player_getter_sender.clone() {
                        Some(mut sender) => {
                            block_on(async {
                                sender.send(()).await.unwrap();
                            });
                            self.player_getter_sender = Some(new_sender)
                        }
                        None => self.player_getter_sender = Some(new_sender),
                    }
                    Task::none()
                }
            },
            // Resultado da verificação de update.
            Message::CheckedUpdates(result) => {
                match result {
                    Ok(url) => {
                        self.update = Update {
                            available: true,
                            url,
                        }
                    }
                    Err(e) => println!("{e}"),
                }
                Task::none()
            }
            Message::OpenLink(url) => {
                open::that(url).unwrap();
                Task::none()
            }
            // Começa a atualização
            Message::Update => {
                self.update.available = false;
                Task::perform(
                    update::install_update(self.update.url.clone()),
                    Message::UpdateResult,
                )
            }
            // Resultado da atualização. Caso houver algum erro, a atualização não vai ser completada.
            Message::UpdateResult(result) => {
                match result {
                    Ok(_) => {
                        let exec_path = env::current_exe().unwrap();

                        let exec_name = exec_path
                            .clone()
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string();

                        // renames current executable to KC-Overlay.old and renames updated executable to KC-Overlay
                        fs::rename(&exec_path, exec_path.with_extension("old")).unwrap();
                        fs::rename(exec_path.with_extension("new"), exec_path).unwrap();

                        let mut new_exe_path = env::current_exe().unwrap();
                        new_exe_path.pop();

                        new_exe_path = new_exe_path.join(exec_name);

                        match std::process::Command::new(new_exe_path).spawn() {
                            Ok(_) => std::process::exit(0),
                            Err(e) => panic!("{}", e),
                        }
                    }
                    Err(e) => {
                        println!("{}", e);
                        Task::none()
                    }
                }
            }
            Message::CustomClientPathModified(path) => {
                Task::perform(async { MineClient::Custom(path) }, Message::ClientSelect)
            }
            Message::SearchExplorer => {
                let file = rfd::FileDialog::new()
                    .add_filter("logs", &["log"])
                    .set_directory(util::get_home_dir());

                let log_path = file.pick_file().unwrap().to_string_lossy().to_string();
                Task::perform(
                    async { MineClient::Custom(log_path) },
                    Message::ClientSelect,
                )
            }
            Message::ChangeNeverMinimize(bool) => {
                self.never_minimize = bool;
                config::save_settings(Some(bool), None);
                Task::none()
            }
            Message::ChangeSecondsToMinimize(f_seconds) => {
                let u_seconds = f_seconds as u64;
                self.seconds_to_minimize = u_seconds;
                config::save_settings(None, Some(u_seconds));
                Task::none()
            },
        }
    }

    // Interface
    fn view(&self) -> Element<Message> {
        screens::get_screen(self.screen, self).into()
    }

    // Gerencia subscriptions. Basicamente código que é executado fora da lógica principal e que tem a capacidade de enviar mensagens. 
    fn subscription(&self) -> Subscription<Message> {
        let event = event::listen().map(Message::GotEvent);
        let logs_reader = Subscription::run(logs_reader).map(Message::Log);

        // A cada 20 segundos atualiza o client do leitor de logs, caso tenha sido mudado.
        let client_updater =
            time::every(Duration::from_secs(20)).map(move |_| Message::ClientUpdate);

        Subscription::batch(vec![event, logs_reader, client_updater])
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }
}

// Output do leitor de logs.
#[derive(Debug, Clone)]
enum LogReader {
    Log(String),
    Sender(mpsc::Sender<MineClient>),
}
// Leitor de logs. Envia toda linha dos logs para a lógica principal, com o objetivo de obter a lista de jogadores.
fn logs_reader() -> impl Stream<Item = LogReader> {
    stream::channel(100, |mut output| async move {
        // comunicação entre a lógica principal e esta stream.
        let (sender, mut receiver) = mpsc::channel(100);
        output.send(LogReader::Sender(sender)).await.unwrap();

        let client = receiver.select_next_some().await;
        let logs_path = get_logs_path(client);
        let mut file = File::open(&logs_path);

        /*
        * Se o arquivo de logs existir, tudo certo. Caso contrário, espera a lógica principal enviar um que exista.
        * O usuário pode selecionar um client que ele não tenha instalado ou colocar um custom client que não exista,
        * fazendo o programa procurar por um log inexistente.
        */
        match file {
            Ok(ok) => {
                file = Ok(ok);
            }
            Err(_) => {
                while !Path::new(&logs_path).exists() {
                    match receiver.try_next() {
                        Ok(Some(message)) => {
                            let logs_path = get_logs_path(message);

                            match File::open(&logs_path) {
                                Ok(ok) => {
                                    file = Ok(ok);
                                    break;
                                }
                                Err(e) => {
                                    println!("{e}");
                                    continue;
                                }
                            }
                        }
                        Ok(None) => {
                            continue;
                        }
                        Err(_) => {
                            continue;
                        }
                    }
                }
            }
        }

        let mut reader = BufReader::new(file.unwrap());
        let mut buffer = String::new();
        reader.seek(SeekFrom::End(0)).unwrap();

        // Lê e envia pra lógica principal.
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

            // Verifica se a lógica principal pediu para atualizar o client.
            match receiver.try_next() {
                Ok(Some(message)) => {
                    let logs_path = get_logs_path(message);

                    let file = match File::open(&logs_path) {
                        Ok(ok) => ok,
                        Err(e) => {
                            println!("{e}");
                            continue;
                        }
                    };

                    reader = BufReader::new(file);
                    buffer = String::new();
                    reader.seek(SeekFrom::End(0)).unwrap();
                }
                Ok(None) => (),
                Err(_) => (),
            }
        }
    })
}

fn get_logs_path(client: MineClient) -> String{
    let minecraft_dir = util::get_minecraft_dir();

    match client {
        MineClient::Default => format!("{}/logs/latest.log", minecraft_dir),
        MineClient::Badlion => {
            format!("{}/logs/blclient/minecraft/latest.log", minecraft_dir)
        }
        MineClient::Lunar => util::lunar_get_newer_logs_path(),
        MineClient::LegacyLauncher => util::get_legacy_launcher_dir(),
        MineClient::Custom(path) => path,
        MineClient::Silent => {
            format!("{}/silentclient/logs/main.log", util::get_home_dir())
        }
    }
}

// Output do código responsável por obter os stats dos players.
#[derive(Clone, Debug)]
enum PlayerSender {
    Player(Player),
    Sender(mpsc::Sender<()>),
    Done,
}

// Pega os stats dos players da API do Mush.
fn get_players(str_player_list: Vec<String>) -> impl Stream<Item = PlayerSender> {
    stream::channel(100, |mut output| async move {
        let (sender, mut receiver) = mpsc::channel(100);

        output.send(PlayerSender::Sender(sender)).await.unwrap();
        let client = Client::new();
        const MUSH_API: &str = "https://mush.com.br/api/player/";

        let mut interrupted = false;

        for i in str_player_list {
            let url = format!("{}{}", MUSH_API, i);
            let request = match client.get(url).send().await {
                Ok(response) => match response.text().await {
                    Ok(ok) => ok,
                    Err(e) => {
                        println!("Failed to get text of {i}'s API response: {e}\n Skipping.");
                        continue;
                    }
                },
                Err(e) => {
                    println!("Failed to get {i} response: {e}\n Skipping.");
                    continue;
                }
            };

            println!("Getting {i} stats...");

            let json: Value = match serde_json::from_str(&request) {
                Ok(ok) => ok,
                Err(e) => {
                    println!("{i}: {e}");
                    continue;
                }
            };

            if !json["success"].as_bool().unwrap() {
                output
                    .send(PlayerSender::Player(Player::new_nicked(i.to_string())))
                    .await
                    .unwrap();
                continue;
            }
            let response = json["response"].clone();

            if response["last_login"].as_i64().unwrap() - response["first_login"].as_i64().unwrap()
                < 7200000
            {
                output
                    .send(PlayerSender::Player(Player::new_possible_cheater(
                        i.to_string(),
                    )))
                    .await
                    .unwrap();
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
                let level_symbol_raw: String = bedwars_stats["level_badge"]["format"]
                    .as_str()
                    .unwrap()
                    .to_string();

                let level_symbol = level_symbol_raw
                    .chars()
                    .find(|c| {
                        !c.is_ascii_alphanumeric()
                            && !c.is_ascii_whitespace()
                            && !c.is_ascii_punctuation()
                    })
                    .unwrap()
                    .to_string();

                let level_color = bedwars_stats["level_badge"]["hex_color"]
                    .as_str()
                    .unwrap_or("#ffffff")
                    .to_string();

                let winstreak = bedwars_stats["winstreak"].as_i64().unwrap_or(0);

                let winrate = bedwars_stats["wins"].as_i64().unwrap_or(0) as f32
                    / bedwars_stats["losses"].as_i64().unwrap_or(0) as f32;
                let final_kill_final_death_ratio =
                    bedwars_stats["final_kills"].as_i64().unwrap_or(0) as f32
                        / bedwars_stats["final_deaths"].as_i64().unwrap_or(0) as f32;

                output
                    .send(PlayerSender::Player(Player::new(
                        i,
                        Rgb::from_hex(username_color),
                        level as i32,
                        level_symbol,
                        winstreak as i32,
                        clan,
                        Rgb::from_hex(clan_color),
                        winrate,
                        final_kill_final_death_ratio,
                        Rgb::from_hex(&level_color),
                    )))
                    .await
                    .unwrap();
            }
            /*
             * Verifica se a lógica principal quer parar a obtenção de stats.
             * Isso acontece quando o jogador digita /jogando enquanto este código está sendo executado.
            */ 
            if receiver.try_next().is_ok() {
                interrupted = true;
                break;
            }
            // Espera um tempo antes de conseguir os stats do próximo player. Isso é pra não saturar a API do Mush.
            sleep(Duration::from_millis(50)).await;
        }
        if !interrupted {
            output.send(PlayerSender::Done).await.unwrap();
        }
    })
}

async fn update_client(mut sender: Sender<MineClient>, client: MineClient) {
    sender.send(client).await.unwrap();
}

// Estrutura dos stats de um player
#[derive(Debug, Clone)]
struct Player {
    username: String,
    username_color: Rgb,
    level: i32,
    level_symbol: String,
    winstreak: i32,
    clan: Option<String>,
    clan_color: Rgb,
    is_nicked: bool,
    is_possible_cheater: bool,
    winrate: f32,
    final_kill_final_death_ratio: f32,
    level_color: Rgb,
}

// Funções para construir uma estrutura de player
impl Player {
    fn new(
        username: String,
        username_color: Rgb,
        level: i32,
        level_symbol: String,
        winstreak: i32,
        clan: Option<String>,
        clan_color: Rgb,
        winrate: f32,
        final_kill_final_death_ratio: f32,
        level_color: Rgb,
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
            username_color: Rgb::new(0, 255, 255),
            level: 999,
            level_symbol: "?".to_string(),
            winstreak: 0,
            clan: None,
            clan_color: Rgb::new(0, 0, 0),
            is_nicked: true,
            is_possible_cheater: false,
            winrate: 0.,
            final_kill_final_death_ratio: 0.,
            level_color: Rgb::new(0, 255, 255),
        }
    }

    fn new_possible_cheater(username: String) -> Self {
        Player {
            username,
            username_color: Rgb::new(255, 0, 0),
            level: 999,
            level_symbol: "?".to_string(),
            winstreak: 0,
            clan: None,
            clan_color: Rgb::new(0, 0, 0),
            is_nicked: false,
            is_possible_cheater: true,
            winrate: 0.,
            final_kill_final_death_ratio: 0.,
            level_color: Rgb::new(0, 255, 255),
        }
    }
}

// Clients
#[derive(Default, Clone, Debug, PartialEq)]
enum MineClient {
    #[default]
    Default,
    Badlion,
    Lunar,
    LegacyLauncher,
    Custom(String),
    Silent,
}

// Clients em string.
impl Display for MineClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MineClient::Default => write!(f, "Geral"),
            MineClient::Badlion => write!(f, "Badlion"),
            MineClient::Lunar => write!(f, "Lunar"),
            MineClient::LegacyLauncher => write!(f, "Legacy Launcher"),
            MineClient::Custom(_) => write!(f, "Personalizado"),
            MineClient::Silent => write!(f, "Silent Client"),
        }
    }
}

// Estrutura de um update.
#[derive(Default)]
struct Update {
    available: bool,
    url: String,
}

impl Update {
    fn empty() -> Self {
        Self {
            available: false,
            url: String::new(),
        }
    }
}
