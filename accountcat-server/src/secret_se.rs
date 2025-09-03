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
