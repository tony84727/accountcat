use secrecy::SecretString;
use serde::Serializer;

pub fn serialize_secret<S>(_: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str("<hidden>")
}

pub fn serialize_optional_secret<S>(
    secret: &Option<SecretString>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(match secret {
        Some(_) => "<hidden>",
        None => "None",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestStruct {
        #[serde(serialize_with = "serialize_secret")]
        secret: SecretString,
        #[serde(serialize_with = "serialize_optional_secret")]
        optional_secret_some: Option<SecretString>,
        #[serde(serialize_with = "serialize_optional_secret")]
        optional_secret_none: Option<SecretString>,
    }

    #[test]
    fn test_serialization() {
        let test_struct = TestStruct {
            secret: SecretString::from(String::from("password123")),
            optional_secret_some: Some(SecretString::from(String::from("secret456"))),
            optional_secret_none: None,
        };

        let serialized = toml::to_string(&test_struct).unwrap();

        assert!(serialized.contains("secret = \"<hidden>\""));
        assert!(serialized.contains("optional_secret_some = \"<hidden>\""));
        assert!(serialized.contains("optional_secret_none = \"None\""));

        // Ensure the actual secrets are not present in the serialized output
        assert!(!serialized.contains("password123"));
        assert!(!serialized.contains("secret456"));
    }
}
