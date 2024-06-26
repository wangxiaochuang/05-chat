use std::{env, fs::File};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AuthConfig {
    pub pk: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub db_url: String,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        // reqad from /etc/config/app.yml or ./app.yml or from env CHAT_CONFIG
        let ret = match (
            File::open("./notify.yml"),
            File::open("/etc/config/notify.yml"),
            env::var("NOTIFY_CONFIG"),
        ) {
            (Ok(reader), _, _) => serde_yaml::from_reader(reader),
            (_, Ok(reader), _) => serde_yaml::from_reader(reader),
            (_, _, Ok(path)) => serde_yaml::from_reader(File::open(path)?),
            _ => bail!("no config file found"),
        };
        Ok(ret?)
    }

    pub fn load_from_reader<R: std::io::Read>(reader: R) -> Result<Self> {
        Ok(serde_yaml::from_reader(reader)?)
    }
}
