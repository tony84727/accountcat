use crate::secret_se::{serialize_optional_secret, serialize_secret};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub login: Login,
    pub database: Option<Database>,
}

impl Config {
    pub fn dump(&self) -> String {
        toml::to_string_pretty(self).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Login {
    #[serde(serialize_with = "serialize_secret")]
    pub client_id: SecretString,
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
    #[serde(serialize_with = "serialize_optional_secret")]
    password: Option<SecretString>,
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
            options = options.password(password.expose_secret());
        }
        options
    }
}

impl From<Database> for PgPool {
    fn from(value: Database) -> Self {
        let connection = PgConnectOptions::from(value);
        PgPoolOptions::new().connect_lazy_with(connection)
    }
}
pub fn print_settings() {
    let config = load().unwrap();
    println!("{}", config.dump());
}
