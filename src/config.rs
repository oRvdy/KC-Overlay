use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use serde_json::Value;

pub fn get_config_file_path() -> String {
    return format!(
        "{}/kc_overlay_config.json",
        super::util::get_minecraft_dir()
    );
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
    }

    let serializedjson = serde_json::to_string_pretty(&conf_json).unwrap();

    file.write_all(serializedjson.as_bytes()).unwrap();

    !file_exists
}
