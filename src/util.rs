use std::{
    fs::{self, File},
    io::Read,
    path::Path,
    time::{Duration, SystemTime},
};

use iced::Color;
use serde_json::Value;
use tokio::time::sleep;

pub fn get_home_dir() -> String {
    match std::env::consts::OS {
        "linux" => format!("{}", std::env::var("HOME").unwrap()),
        "windows" => format!(
            "{}/AppData/Roaming",
            std::env::var("USERPROFILE").unwrap().replace('\\', "/")
        ),
        _ => panic!("System not supported."),
    }
}
pub fn get_minecraft_dir() -> String {
    format!("{}/.minecraft", get_home_dir())
}

pub fn get_legacy_launcher_dir() -> String {
    match std::env::consts::OS {
        "linux" => format!(
            "{}/.tlauncher/legacy/Minecraft/game/logs/latest.log",
            std::env::var("HOME").unwrap()
        ),
        "windows" => format!(
            "{}/AppData/Roaming/.tlauncher/legacy/Minecraft/game/logs/latest.log",
            std::env::var("USERPROFILE").unwrap().replace('\\', "/")
        ),
        _ => panic!("System not supported."),
    }
}

pub async fn wait(time: Duration) -> bool {
    sleep(time).await;
    true
}

pub fn get_json(jpathstring: String) -> Value {
    let jsonpath = Path::new(&jpathstring);

    let mut file = File::open(jsonpath).unwrap();
    let mut fcontent = String::new();
    file.read_to_string(&mut fcontent).unwrap();
    serde_json::from_str(&fcontent).unwrap()
}

pub fn lunar_get_newer_logs_path() -> String {
    let lunar_dir = match std::env::consts::OS {
        "linux" => format!("{}/.lunarclient", std::env::var("HOME").unwrap()),
        "windows" => format!(
            "{}/.lunarclient",
            std::env::var("USERPROFILE").unwrap().replace('\\', "/")
        ),
        _ => panic!("System not supported."),
    };

    let logs_path = format!("{}/logs/game", lunar_dir);

    fs::create_dir_all(logs_path.clone()).unwrap();

    let mut newer_log_path = String::new();
    let mut shortest_modification_time = SystemTime::UNIX_EPOCH;
    for log in fs::read_dir(logs_path).unwrap() {
        let log = log.unwrap();
        let metadata = log.metadata().unwrap();

        let modified_time = metadata.modified().unwrap();
        if modified_time > shortest_modification_time {
            newer_log_path = log.path().to_string_lossy().to_string();
            shortest_modification_time = modified_time;
        }
    }

    println!("{}", newer_log_path);
    newer_log_path
}

#[derive(Debug, Clone)]
pub struct RGB {
    red: u8,
    green: u8,
    blue: u8,
}

impl RGB {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        RGB {
            red: r,
            green: g,
            blue: b,
        }
    }
    pub fn from_hex(hex: &str) -> Self {
        let hex = hex.replace("#", "");

        let red = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let green = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let blue = u8::from_str_radix(&hex[4..6], 16).unwrap();

        RGB { red, green, blue }
    }

    pub fn to_color(&self) -> Color {
        Color::from_rgb8(self.red, self.green, self.blue)
    }
}
