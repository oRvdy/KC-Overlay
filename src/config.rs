use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
};

use serde_json::Value;

pub fn get_config_file_path() -> String {
    format!(
        "{}/kc_overlay_config.json",
        super::util::get_minecraft_dir()
    )
}

pub fn get_config() -> Value {
    super::util::get_json(get_config_file_path())
}

pub fn check_config_file() -> bool {
    let file_exists = Path::new(&get_config_file_path()).exists();
    let mut conf_json = match file_exists {
        true => super::util::get_json(get_config_file_path()),
        false => {
            let minecraft_path = super::util::get_minecraft_dir();
            if !Path::new(&minecraft_path).exists() {
                fs::create_dir_all(minecraft_path).unwrap()
            }
            serde_json::json!({})
        }
    };

    let mut file = File::create(get_config_file_path()).unwrap();

    if let Value::Object(map) = &mut conf_json {
        if !map.contains_key("client") {
            map.insert("client".to_owned(), serde_json::to_value(0).unwrap());
        }
        if !map.contains_key("custom_client_path") {
            map.insert(
                "custom_client_path".to_owned(),
                serde_json::to_value("").unwrap(),
            );
        }
        if !map.contains_key("never_minimize") {
            map.insert(
                "never_minimize".to_owned(),
                serde_json::to_value(false).unwrap(),
            );
        }
        if !map.contains_key("seconds_to_minimize") {
            map.insert(
                "seconds_to_minimize".to_owned(),
                serde_json::to_value(12).unwrap(),
            );
        }
        if !map.contains_key("remove_eliminated_players") {
            map.insert(
                "remove_eliminated_players".to_owned(),
                serde_json::to_value(true).unwrap(),
            );
        }
    }

    let serializedjson = serde_json::to_string_pretty(&conf_json).unwrap();

    file.write_all(serializedjson.as_bytes()).unwrap();

    !file_exists
}

pub fn save_settings(
    never_minimize: Option<bool>,
    seconds_to_minimize: Option<u64>,
    remove_eliminated_players: Option<bool>,
) {
    let mut config = get_config();

    if let Some(never_minimize_option) = never_minimize {
        config["never_minimize"] = serde_json::json!(never_minimize_option)
    }
    if let Some(seconds) = seconds_to_minimize {
        config["seconds_to_minimize"] = serde_json::json!(seconds)
    }
    if let Some(remove_eliminated_players_option) = remove_eliminated_players {
        config["remove_eliminated_players"] = serde_json::json!(remove_eliminated_players_option)
    }

    let mut config_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(get_config_file_path())
        .unwrap();
    config_file
        .write_all(serde_json::to_string_pretty(&config).unwrap().as_bytes())
        .unwrap();
}
