use clap::Parser;
use sha1::{Digest, Sha1};
use std::env;
use std::fs::{self, File};
use std::io;
use std::path::Path;

#[derive(Parser)]
struct Args {
    #[clap(long, num_args = 1.., required = true)]
    packages: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let packages = load_packages(env::current_dir()?)?;
    for package_name in &args.packages {
        let package = packages
            .iter()
            .find(|package| package.name == *package_name)
            .ok_or_else(|| anyhow::format_err!("missing package `{package_name}`"))?;
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

fn load_packages<P>(root: P) -> anyhow::Result<Vec<package::Package>>
where
    P: AsRef<Path>,
{
    #[tracing::instrument(err, ret)]
    fn load(path: &Path) -> anyhow::Result<package::Package> {
        let mut package = toml::from_str::<package::Package>(&fs::read_to_string(path)?)?;
        for file in &mut package.files {
            if file.source.is_relative() {
                file.source = path
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
        Ok(package)
    }

    let mut packages = Vec::new();
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry?;
        if entry.file_name() == concat!(env!("CARGO_BIN_NAME"), ".toml") {
            packages.push(load(entry.path())?)
        }
    }
    Ok(packages)
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

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct State {
        pub files: BTreeMap<PathBuf, File>,
    }

    #[serde_with::serde_as]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct File {
        pub owner: String,
        pub mode: u32,
        #[serde_as(as = "serde_with::hex::Hex")]
        pub sha1: [u8; 20],
        #[serde(skip_serializing_if = "Option::is_none")]
        pub post_install: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub pre_remove: Option<String>,
    }
}
