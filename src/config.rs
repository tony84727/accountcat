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
    pub database: Database,
}

impl Config {
    pub fn dump(&self) -> String {
        toml::to_string_pretty(self).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct ConfigFile {
    pub login: Option<Login>,
    pub database: Option<Database>,
}

#[derive(Serialize, Deserialize)]
pub struct Login {
    #[serde(serialize_with = "serialize_secret")]
    pub client_id: SecretString,
}

pub fn load() -> Result<Config, LoadError> {
    let config_file: Option<ConfigFile> = std::fs::read_to_string("server.toml")
        .ok()
        .and_then(|content| toml::from_str(&content).ok());
    let (login, database) = match config_file {
        Some(database) => (database.login, database.database),
        None => (None, None),
    };
    let login: Login = std::env::var("GOOGLE_LOGIN_CLIENT_ID")
        .ok()
        .map(|client_id| Login {
            client_id: SecretString::from(client_id),
        })
        .or(login)
        .ok_or(LoadError::MissingEssentialValue("login.client_id"))?;
    let database = Database::from_env()
        .or(database)
        .or(Some(Default::default()));
    Ok(Config { login, database })
}

#[derive(Debug)]
pub enum LoadError {
    IO(std::io::Error),
    Parse(toml::de::Error),
    MissingEssentialValue(&'static str),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Database {
    pub host: Option<String>,
    pub user: Option<String>,
    #[serde(serialize_with = "serialize_optional_secret")]
    pub password: Option<SecretString>,
    pub database: Option<String>,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            host: Some(String::from("localhost")),
            user: Some(String::from("postgres")),
            password: None,
            database: Some(String::from("accountcat")),
        }
    }
}

impl Database {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("DATABASE_HOST").ok(),
            user: std::env::var("DATABASE_USER").ok(),
            password: std::env::var("DATABASE_PASSWORD")
                .map(SecretString::from)
                .ok(),
            database: std::env::var("DATABASE_NAME").ok(),
        }
    }

    pub fn or(self, other: Option<Self>) -> Self {
        let Some(other) = other else {
            return self;
        };
        let Database {
            host,
            user,
            password,
            database,
        } = self;
        Self {
            host: host.or(other.host),
            user: user.or(other.user),
            password: password.or(other.password),
            database: database.or(other.database),
        }
    }
}

impl From<Database> for PgConnectOptions {
    fn from(value: Database) -> Self {
        let Database {
            host,
            user,
            password,
            database,
        } = value.or(Some(Default::default()));
        let mut options = PgConnectOptions::new()
            .host(&host.unwrap())
            .username(&user.unwrap())
            .database(&database.unwrap());
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
