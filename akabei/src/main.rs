use sha1::{Digest, Sha1};
use std::env;
use std::fs::{self, File};
use std::io;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let mut packages = Vec::new();
    for entry in walkdir::WalkDir::new(env::current_dir()?) {
        let entry = entry?;
        if entry.file_name() == concat!(env!("CARGO_BIN_NAME"), ".toml") {
            let mut package =
                toml::from_str::<package::Package>(&fs::read_to_string(entry.path())?)?;
            for file in &mut package.files {
                if file.source.is_relative() {
                    file.source = entry
                        .path()
                        .parent()
                        .ok_or_else(|| anyhow::format_err!("missing parent"))?
                        .join(&file.source);
                }
                if file.target.is_relative() {
                    file.target = dirs::home_dir()
                        .ok_or_else(|| anyhow::format_err!("missing home_dir"))?
                        .join(&file.target);
                }
            }
            packages.push(package);
        }
    }

    for package in &packages {
        for file in &package.files {
            #[tracing::instrument(err, ret)]
            fn copy(source: &Path, target: &Path) -> io::Result<()> {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(source, target)?;
                Ok(())
            }
            copy(&file.source, &file.target)?;
        }
    }

    Ok(())
}

fn sha1<P>(path: P) -> io::Result<[u8; 20]>
where
    P: AsRef<Path>,
{
    let mut hasher = Sha1::new();
    io::copy(&mut File::open(path)?, &mut hasher)?;
    Ok(hasher.finalize().into())
}

mod package {
    use serde::Deserialize;
    use std::path::PathBuf;

    #[derive(Debug, Deserialize)]
    pub struct Package {
        pub name: String,
        pub files: Vec<File>,
    }

    #[derive(Debug, Deserialize)]
    pub struct File {
        pub source: PathBuf,
        pub target: PathBuf,
        pub post_install: Option<String>,
        pub pre_remove: Option<String>,
    }
}

mod state {
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[derive(Debug, Deserialize, Serialize)]
    pub struct State {
        pub files: BTreeMap<PathBuf, File>,
    }

    #[serde_with::serde_as]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct File {
        #[serde_as(as = "serde_with::hex::Hex")]
        pub sha1: [u8; 20],
        #[serde(skip_serializing_if = "Option::is_none")]
        pub pre_remove: Option<String>,
    }
}
