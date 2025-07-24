use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub login: Login,
}

#[derive(Serialize, Deserialize)]
pub struct Login {
    pub client_id: String,
}

pub fn load() -> Result<Config, LoadError> {
    toml::from_str(&std::fs::read_to_string("server.toml").map_err(LoadError::IO)?)
        .map_err(LoadError::Parse)
}

#[derive(Debug)]
pub enum LoadError {
    IO(std::io::Error),
    Parse(toml::de::Error),
}
