use std::time::Duration;

use iced::{
    futures::{channel::mpsc, SinkExt, Stream},
    stream,
};
use reqwest::Client;
use serde_json::Value;
use tokio::time::sleep;

use crate::{
    stats::{Bedwars, Stats, StatsType},
    util::Rgb,
    PlayerSender,
};

// Estrutura dos stats de um player
#[derive(Debug, Clone)]
pub struct Player {
    pub username: String,
    pub username_color: Rgb,
    pub clan: Option<String>,
    pub clan_color: Rgb,
    pub is_nicked: bool,
    pub is_possible_cheater: bool,
    pub account_creation: i64,
    pub last_login: i64,
    pub is_connected: bool,
    pub stats: Stats,
}

// Funções para construir uma estrutura de player
impl Player {
    pub fn new(
        username: String,
        username_color: Rgb,
        clan: Option<String>,
        clan_color: Rgb,
        is_possible_cheater: bool,
        account_creation: i64,
        last_login: i64,
        is_connected: bool,
        stats: Stats,
    ) -> Self {
        Player {
            username,
            username_color,
            clan,
            clan_color,
            is_nicked: false,
            is_possible_cheater,
            account_creation,
            last_login,
            is_connected,
            stats,
        }
    }

    pub fn new_nicked(username: String, stats_type: StatsType) -> Self {
        let stats = match stats_type {
            StatsType::BedwarsAll
            | StatsType::BedwarsSolo
            | StatsType::BedwarsDoubles
            | StatsType::BedwarsTrios
            | StatsType::BedwarsQuads => Stats::Bedwars(crate::stats::Bedwars {
                level: 999,
                level_symbol: "?".to_string(),
                winstreak: 0,
                winrate: 0.,
                final_kill_death_ratio: 0.,
                kill_death_ratio: 0.,
                level_color: Rgb::new(0, 255, 255),
                wins: 0,
                losses: 0,
                kills: 0,
                deaths: 0,
                final_kills: 0,
                final_deaths: 0,
                hours_played: 0,
                assists: 0,
            }),
        };
        Player {
            username,
            username_color: Rgb::new(0, 255, 255),
            clan: None,
            clan_color: Rgb::new(0, 0, 0),
            is_nicked: true,
            is_possible_cheater: false,
            stats,
            account_creation: 0,
            last_login: 0,
            is_connected: true,
        }
    }
}

// Pega os stats dos players da API do Mush.
pub fn get_players(
    str_player_list: Vec<String>,
    stats_type: StatsType,
) -> impl Stream<Item = PlayerSender> {
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
                    .send(PlayerSender::Player(Player::new_nicked(
                        i.to_string(),
                        stats_type.clone(),
                    )))
                    .await
                    .unwrap();
                continue;
            }
            let response = json["response"].clone();

            let player = get_player_data(i, response, stats_type.clone());

            output.send(PlayerSender::Player(player)).await.unwrap();
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

// Versão mais simples para pegar os dados de apenas um jogador.
pub async fn get_player(username: &str, stats_type: StatsType) -> Result<Player, ()> {
    let client = Client::new();
    let url = "https://mush.com.br/api/player/".to_string() + username;

    let request = match client.get(url).send().await {
        Ok(response) => match response.text().await {
            Ok(ok) => ok,
            Err(e) => {
                println!("Failed to get text of {username}'s API response: {e}\n Skipping.");
                return Err(());
            }
        },
        Err(e) => {
            println!("Failed to get {username} response: {e}\n Skipping.");
            return Err(());
        }
    };

    println!("Getting {username} stats...");

    let json: Value = match serde_json::from_str(&request) {
        Ok(ok) => ok,
        Err(e) => {
            println!("{username}: {e}");
            return Err(());
        }
    };

    if !json["success"].as_bool().unwrap() {
        return Ok(Player::new_nicked(username.to_owned(), stats_type));
    }
    let response = json["response"].clone();

    Ok(get_player_data(username.to_owned(), response, stats_type))
}

fn get_player_data(username: String, response: Value, stats_type: StatsType) -> Player {
    let is_possible_cheater = response["last_login"].as_i64().unwrap()
        - response["first_login"].as_i64().unwrap()
        < 7200000;

    let username_color = response["rank_tag"]["color"].as_str().unwrap();
    let (clan, clan_color) = if response["clan"].is_object() {
        (
            Some(response["clan"]["tag"].as_str().unwrap().to_string()),
            response["clan"]["tag_color"].as_str().unwrap(),
        )
    } else {
        (None, "#ffffff")
    };

    let account_creation = response["first_login"].as_i64().unwrap();
    let last_login = response["last_login"].as_i64().unwrap();
    let is_connected = response["connected"].as_bool().unwrap();

    let stats = match stats_type {
        StatsType::BedwarsAll
        | StatsType::BedwarsSolo
        | StatsType::BedwarsDoubles
        | StatsType::BedwarsTrios
        | StatsType::BedwarsQuads => {
            let bedwars_stats = response["stats"]["bedwars"].clone();
            let level = if !is_possible_cheater {
                bedwars_stats["level"].as_i64().unwrap_or(0)
            } else {
                998
            };
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

            let level_color = level_symbol_raw.chars().nth(1).unwrap();

            let (
                ws_entry,
                wins_entry,
                losses_entry,
                kills_entry,
                deaths_entry,
                final_kills_entry,
                final_deaths_entry,
                assists_entry,
                hours_played_entry,
            ) = match stats_type {
                StatsType::BedwarsAll => (
                    "winstreak",
                    "wins",
                    "losses",
                    "kills",
                    "deaths",
                    "final_kills",
                    "final_deaths",
                    "assists",
                    "bedwars",
                ),
                StatsType::BedwarsSolo => (
                    "solo_winstreak",
                    "solo_wins",
                    "solo_losses",
                    "solo_kills",
                    "solo_deaths",
                    "solo_final_kills",
                    "solo_final_deaths",
                    "solo_assists",
                    "bedwars_solo",
                ),
                StatsType::BedwarsDoubles => (
                    "doubles_winstreak",
                    "doubles_wins",
                    "doubles_losses",
                    "doubles_kills",
                    "doubles_deaths",
                    "doubles_final_kills",
                    "doubles_final_deaths",
                    "doubles_assists",
                    "bedwars_doubles",
                ),
                StatsType::BedwarsTrios => (
                    "3v3v3v3_winstreak",
                    "3v3v3v3_wins",
                    "3v3v3v3_losses",
                    "3v3v3v3_kills",
                    "3v3v3v3_deaths",
                    "3v3v3v3_final_kills",
                    "3v3v3v3_final_deaths",
                    "3v3v3v3_assists",
                    "bedwars_3v3v3v3",
                ),
                StatsType::BedwarsQuads => (
                    "4v4v4v4_winstreak",
                    "4v4v4v4_wins",
                    "4v4v4v4_losses",
                    "4v4v4v4_kills",
                    "4v4v4v4_deaths",
                    "4v4v4v4_final_kills",
                    "4v4v4v4_final_deaths",
                    "4v4v4v4_assists",
                    "bedwars_4v4v4v4",
                ),
                //_ => panic!("Impossível!"),
            };

            let winstreak = bedwars_stats[ws_entry].as_i64().unwrap_or(0) as i32;

            let mut winrate = bedwars_stats[wins_entry].as_i64().unwrap_or(0) as f32
                / bedwars_stats[losses_entry].as_i64().unwrap_or(0) as f32;
            let mut final_kill_death_ratio = bedwars_stats[final_kills_entry].as_i64().unwrap_or(0)
                as f32
                / bedwars_stats[final_deaths_entry].as_i64().unwrap_or(0) as f32;

            let mut kill_death_ratio = bedwars_stats[kills_entry].as_i64().unwrap_or(0) as f32
                / bedwars_stats[deaths_entry].as_i64().unwrap_or(0) as f32;

            if winrate.is_nan() || winrate.is_infinite() {
                winrate = 0.0;
            }
            if final_kill_death_ratio.is_nan() || final_kill_death_ratio.is_infinite() {
                final_kill_death_ratio = 0.0;
            }
            if kill_death_ratio.is_nan() || kill_death_ratio.is_infinite() {
                kill_death_ratio = 0.0;
            }

            let wins = bedwars_stats[wins_entry].as_u64().unwrap_or(0);
            let losses = bedwars_stats[losses_entry].as_u64().unwrap_or(0);
            let kills = bedwars_stats[kills_entry].as_u64().unwrap_or(0);
            let deaths = bedwars_stats[deaths_entry].as_u64().unwrap_or(0);
            let final_kills = bedwars_stats[final_kills_entry].as_u64().unwrap_or(0);
            let final_deaths = bedwars_stats[final_deaths_entry].as_u64().unwrap_or(0);
            let assists = bedwars_stats[assists_entry].as_u64().unwrap_or(0);
            let hours_played = response["stats"]["play_time"][hours_played_entry]
                .as_u64()
                .unwrap_or(1)
                / 3600;

            Stats::Bedwars(Bedwars {
                level: level as i32,
                level_symbol,
                winstreak,
                winrate,
                final_kill_death_ratio,
                kill_death_ratio,
                level_color: Rgb::from_minecraft_color(&level_color),
                wins,
                losses,
                kills,
                deaths,
                final_kills,
                final_deaths,
                hours_played,
                assists,
            })
        }
    };

    Player::new(
        username,
        Rgb::from_hex(username_color),
        clan,
        Rgb::from_hex(clan_color),
        is_possible_cheater,
        account_creation,
        last_login,
        is_connected,
        stats,
    )
}
