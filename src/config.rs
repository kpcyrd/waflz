use crate::errors::*;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    pub irc: Config,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub nickname: String,
    pub password: Option<String>,

    pub server: String,
    pub port: Option<u16>,

    pub channels: Vec<String>,
    #[serde(default)]
    pub readonly_channels: HashSet<String>,
}

pub fn load_from(path: &str) -> Result<ConfigFile> {
    let buf = fs::read_to_string(path).context("Failed to read config file")?;
    let config = toml::from_str(&buf).context("Failed to deserialize config")?;
    Ok(config)
}
