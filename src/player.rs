use std::time::Duration;

use iced::{
    futures::{channel::mpsc, SinkExt, Stream},
    stream,
};
use reqwest::Client;
use serde_json::Value;
use tokio::time::sleep;

use crate::{util::Rgb, PlayerSender};

// Estrutura dos stats de um player
#[derive(Debug, Clone)]
pub struct Player {
    pub username: String,
    pub username_color: Rgb,
    pub level: i32,
    pub level_symbol: String,
    pub winstreak: i32,
    pub clan: Option<String>,
    pub clan_color: Rgb,
    pub is_nicked: bool,
    pub is_possible_cheater: bool,
    pub winrate: f32,
    pub final_kill_final_death_ratio: f32,
    pub kill_death_ratio: f32,
    pub level_color: Rgb,
}

// Funções para construir uma estrutura de player
impl Player {
    pub fn new(
        username: String,
        username_color: Rgb,
        level: i32,
        level_symbol: String,
        winstreak: i32,
        clan: Option<String>,
        clan_color: Rgb,
        is_possible_cheater: bool,
        winrate: f32,
        final_kill_final_death_ratio: f32,
        kill_death_ratio: f32,
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
            is_possible_cheater,
            winrate,
            final_kill_final_death_ratio,
            kill_death_ratio,
            level_color,
        }
    }

    pub fn new_nicked(username: String) -> Self {
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
            kill_death_ratio: 0.,
            level_color: Rgb::new(0, 255, 255),
        }
    }
}

// Pega os stats dos players da API do Mush.
pub fn get_players(str_player_list: Vec<String>) -> impl Stream<Item = PlayerSender> {
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

            let player = get_player_data(i, response);

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
pub async fn get_player(username: &str) -> Result<Player, ()> {
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
        return Ok(Player::new_nicked(username.to_owned()));
    }
    let response = json["response"].clone();

    Ok(get_player_data(username.to_owned(), response))
}

fn get_player_data(username: String, response: Value) -> Player {
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
            !c.is_ascii_alphanumeric() && !c.is_ascii_whitespace() && !c.is_ascii_punctuation()
        })
        .unwrap()
        .to_string();

    let level_color = level_symbol_raw.chars().nth(1).unwrap();

    let winstreak = bedwars_stats["winstreak"].as_i64().unwrap_or(0);

    let mut winrate = bedwars_stats["wins"].as_i64().unwrap_or(0) as f32
        / bedwars_stats["losses"].as_i64().unwrap_or(0) as f32;
    let mut final_kill_final_death_ratio = bedwars_stats["final_kills"].as_i64().unwrap_or(0)
        as f32
        / bedwars_stats["final_deaths"].as_i64().unwrap_or(0) as f32;

    let mut kill_death_ratio = bedwars_stats["kills"].as_i64().unwrap_or(0) as f32
        / bedwars_stats["deaths"].as_i64().unwrap_or(0) as f32;

    if winrate.is_nan() || winrate.is_infinite() {
        winrate = 0.0;
    }
    if final_kill_final_death_ratio.is_nan() || final_kill_final_death_ratio.is_infinite() {
        final_kill_final_death_ratio = 0.0;
    }
    if kill_death_ratio.is_nan() || kill_death_ratio.is_infinite() {
        kill_death_ratio = 0.0;
    }

    Player::new(
        username,
        Rgb::from_hex(username_color),
        level as i32,
        level_symbol,
        winstreak as i32,
        clan,
        Rgb::from_hex(clan_color),
        is_possible_cheater,
        winrate,
        final_kill_final_death_ratio,
        kill_death_ratio,
        Rgb::from_minecraft_color(&level_color),
    )
}
