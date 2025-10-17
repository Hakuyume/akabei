use crate::misc;
use serde::{Deserialize, Deserializer, Serialize, de};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(bound(deserialize = "T: Default"))]
pub struct State<T> {
    pub packages: Vec<Package<T>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(bound(deserialize = "T: Default"))]
pub struct Package<T> {
    pub name: String,
    pub files: BTreeMap<PathBuf, File<T>>,
    #[serde(default)]
    pub hooks: Hooks,
}

#[serde_with::serde_as]
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct File<T> {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub sha1: [u8; 20],
    #[serde_as(as = "misc::Octal")]
    pub mode: u32,
    #[serde(skip)]
    pub extra: T,
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Hooks {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pre_install: Vec<Hook>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_install: Vec<Hook>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pre_remove: Vec<Hook>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_remove: Vec<Hook>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hook {
    #[serde(deserialize_with = "deserialize_command")]
    pub command: Vec<String>,
}

fn deserialize_command<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Vec<String>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a sequence")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(["sh", "-euxc", value].into_iter().map(Into::into).collect())
        }

        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }
    }

    deserializer.deserialize_any(Visitor)
}
