use crate::misc;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(bound(deserialize = "T: Default"))]
pub struct Manifest<T> {
    pub packages: BTreeSet<String>,
    pub files: BTreeMap<PathBuf, File<T>>,
}

#[serde_with::serde_as]
#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, Serialize)]
pub struct File<T> {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub sha1: [u8; 20],
    #[serde_as(as = "misc::Octal")]
    pub mode: u32,
    pub pre_install: Option<String>,
    pub post_install: Option<String>,
    pub pre_remove: Option<String>,
    pub post_remove: Option<String>,
    #[serde(skip)]
    pub extra: T,
}
