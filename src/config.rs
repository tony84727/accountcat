use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnectOptions;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub login: Login,
    pub database: Option<Database>,
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

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Database {
    host: Option<String>,
    user: Option<String>,
    password: Option<String>,
    database: Option<String>,
}

impl From<Database> for PgConnectOptions {
    fn from(value: Database) -> Self {
        let Database {
            host,
            user,
            password,
            database,
        } = value;
        let mut options = PgConnectOptions::new()
            .host(&host.unwrap_or_else(|| String::from("localhost")))
            .username(&user.unwrap_or_else(|| String::from("postgres")))
            .database(&database.unwrap_or_else(|| String::from("accountcat")));
        if let Some(password) = password {
            options = options.password(&password);
        }
        options
    }
}
