use jsonwebtoken::{DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

pub const DEFAULT_JWK_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";

pub struct JwtVerifier {
    jwk_sets: JwkSet,
    client_id: SecretString,
}

impl JwtVerifier {
    pub async fn new(jwk_url: &str, client_id: SecretString) -> Result<Self, InitError> {
        let jwk_sets: JwkSet = reqwest::get(jwk_url)
            .await
            .map_err(InitError::LoadJwk)?
            .json()
            .await
            .map_err(InitError::LoadJwk)?;
        if jwk_sets.keys.is_empty() {
            return Err(InitError::NotKey);
        }
        Ok(Self {
            jwk_sets,
            client_id,
        })
    }

    pub fn verify(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let header = decode_header(token)?;
        let mut last_error = None;
        for k in self.jwk_sets.keys.iter() {
            let mut validation = Validation::new(header.alg);
            validation.set_audience(&[self.client_id.expose_secret()]);
            match decode::<Claims>(token, &DecodingKey::from_jwk(k)?, &validation) {
                Ok(data) => {
                    return Ok(data.claims);
                }
                Err(err) => {
                    last_error = Some(err);
                }
            };
        }
        Err(last_error.unwrap())
    }
}

#[derive(Debug)]
pub enum InitError {
    LoadJwk(reqwest::Error),
    NotKey,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub azp: String,
    pub aud: String,
    pub sub: String,
    pub iat: u64,
    pub exp: u64,
    pub picture: String,
    pub given_name: String,
    pub family_name: String,
    pub name: String,
}
