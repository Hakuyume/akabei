use crate::{misc, schema};
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Package {
    name: String,
    files: Vec<File>,
    #[serde(default)]
    hooks: schema::Hooks,
}

#[serde_with::serde_as]
#[derive(Debug, Deserialize)]
struct File {
    source: PathBuf,
    target: PathBuf,
    #[serde_as(as = "Option<misc::Octal>")]
    mode: Option<u32>,
    #[serde(default)]
    template: bool,
}

pub fn load_packages<P>(root: P) -> anyhow::Result<BTreeMap<String, schema::Package<Vec<u8>>>>
where
    P: AsRef<Path>,
{
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| match entry {
            Ok(entry) => {
                if entry.file_name() == concat!(env!("CARGO_BIN_NAME"), ".toml") {
                    Some(load_package(entry.path()).map(|package| (package.name.clone(), package)))
                } else {
                    None
                }
            }
            Err(e) => Some(Err(e.into())),
        })
        .collect()
}

#[tracing::instrument(err)]
fn load_package<P>(path: P) -> anyhow::Result<schema::Package<Vec<u8>>>
where
    P: AsRef<Path> + fmt::Debug,
{
    let Package { name, files, hooks } = toml::from_str(&fs::read_to_string(&path)?)?;
    Ok(schema::Package {
        name,
        files: files
            .into_iter()
            .map(|file| load_file(&path, file))
            .collect::<Result<_, _>>()?,
        hooks,
    })
}

#[tracing::instrument(err)]
fn load_file<P>(
    path: P,
    File {
        mut source,
        mut target,
        mode,
        template,
    }: File,
) -> anyhow::Result<(PathBuf, schema::File<Vec<u8>>)>
where
    P: AsRef<Path> + fmt::Debug,
{
    if source.is_relative() {
        source = path
            .as_ref()
            .parent()
            .ok_or_else(|| anyhow::format_err!("missing parent"))?
            .join(&source);
    }
    if target.is_relative() {
        target = dirs::home_dir()
            .ok_or_else(|| anyhow::format_err!("missing home_dir"))?
            .join(&target);
    }
    let content = fs::read(&source)?;
    let content = if template {
        let mut context = tera::Context::new();
        context.insert("env", &env::vars().collect::<BTreeMap<_, _>>());
        context.insert("uid", &nix::unistd::getuid().as_raw());
        tera::Tera::one_off(&String::from_utf8(content)?, &context, false)?.into_bytes()
    } else {
        content
    };
    let sha1 = Sha1::digest(&content).into();
    let mode = mode.unwrap_or(0o100644);
    Ok((
        target,
        schema::File {
            sha1,
            mode,
            extra: content,
        },
    ))
}
