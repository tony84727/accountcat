use std::{path::PathBuf, str::FromStr};

use crate::secret_se::{serialize_optional_secret, serialize_secret};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub general: General,
    pub login: Login,
    pub database: Database,
    pub hashids: HashIds,
    pub pki: Pki,
}

impl Config {
    pub fn dump(&self) -> String {
        toml::to_string_pretty(self).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct ConfigFile {
    pub general: Option<General>,
    pub login: Option<Login>,
    pub database: Option<Database>,
    pub hashids: Option<HashIds>,
    pub pki: Option<Pki>,
}

#[derive(Serialize, Deserialize)]
pub struct Login {
    #[serde(serialize_with = "serialize_secret")]
    pub client_id: SecretString,
}

#[derive(Serialize, Deserialize)]
pub struct HashIds {
    #[serde(serialize_with = "serialize_secret")]
    pub salt: SecretString,
}

#[derive(Serialize, Deserialize, Default)]
pub struct General {
    pub administrators: Option<Vec<String>>,
}

impl General {
    pub fn from_env() -> Self {
        Self {
            administrators: std::env::var("ADMINISTRATORS")
                .ok()
                .map(|a| a.split(",").map(String::from).collect()),
        }
    }

    pub fn or(mut self, other: Option<Self>) -> Self {
        let administrators = other.and_then(|o| o.administrators);
        self.administrators = self.administrators.or(administrators);
        self
    }
}

pub fn load() -> Result<Config, LoadError> {
    load_from_string(std::fs::read_to_string("server.toml").ok())
}

fn load_from_string(config: Option<String>) -> Result<Config, LoadError> {
    let mut config_file: Option<ConfigFile> = None;
    if let Some(config) = config {
        match toml::from_str(&config) {
            Ok(config) => {
                config_file = Some(config);
            }
            Err(err) => return Err(LoadError::Parse(err)),
        }
    }
    let (login, database, hashids, general, pki) = match config_file {
        Some(config_file) => (
            config_file.login,
            config_file.database,
            config_file.hashids,
            config_file.general,
            config_file.pki,
        ),
        None => (None, None, None, None, None),
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
    let hashids = std::env::var("HASHIDS_SALT")
        .ok()
        .map(|salt| HashIds {
            salt: SecretString::from(salt),
        })
        .or(hashids)
        .ok_or(LoadError::MissingEssentialValue("hashids.salt"))?;
    let pki = std::env::var("PKI_CA")
        .ok()
        .map(|directory| Pki {
            ca: PathBuf::from(directory),
        })
        .or(pki)
        .unwrap_or_else(|| Pki::default());
    let general = General::from_env().or(general);
    Ok(Config {
        general,
        login,
        database,
        hashids,
        pki,
    })
}

#[derive(Serialize, Deserialize)]
pub struct Pki {
    pub ca: PathBuf,
}

impl Default for Pki {
    fn default() -> Self {
        Self {
            ca: PathBuf::from_str("./pki").unwrap(),
        }
    }
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

    pub fn without_name(&self) -> Self {
        let mut out = self.clone();
        out.database = None;
        out
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
            .username(&user.unwrap());
        if let Some(database) = database {
            options = options.database(&database);
        }
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

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use crate::config::load_from_string;

    #[test]
    fn test_parse_minimum_config() {
        let toml = r#"
[login]
client_id = "dummy"

[hashids]
salt = "salt"
"#;
        let config = load_from_string(Some(String::from(toml))).unwrap();
        assert_eq!("dummy", config.login.client_id.expose_secret());
        assert_eq!("salt", config.hashids.salt.expose_secret());
        assert!(config.general.administrators.is_none());
    }

    #[test]
    fn test_parse_administrators() {
        let toml = r#"
[general]
administrators = ["a", "b","c"]
[login]
client_id = "dummy"

[hashids]
salt = "salt"

"#;
        let config = load_from_string(Some(String::from(toml))).unwrap();
        assert_eq!(vec!["a", "b", "c"], config.general.administrators.unwrap());
    }
}
