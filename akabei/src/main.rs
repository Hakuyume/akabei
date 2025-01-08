mod manifest;
mod misc;

use clap::Parser;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs::Permissions;
use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
struct Args {
    #[clap(long, num_args = 1..)]
    install: Vec<String>,
    #[clap(long, num_args = 1..)]
    remove: Vec<String>,
    #[clap(long)]
    dry_run: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let state = dirs::data_dir()
        .ok_or_else(|| anyhow::format_err!("missing data_dir"))?
        .join(concat!(env!("CARGO_BIN_NAME"), ".json"));

    let before = if state.try_exists()? {
        serde_json::from_reader::<_, manifest::Manifest<()>>(File::open(&state)?)?
    } else {
        manifest::Manifest::default()
    };

    let mut after = manifest::Manifest::default();

    after.packages = before.packages.clone();
    for package in args.install {
        after.packages.insert(package);
    }
    for package in args.remove {
        after.packages.remove(&package);
    }

    let mut packages = load_packages(env::current_dir()?)?;
    for package_name in &after.packages {
        let manifest = packages
            .iter_mut()
            .find(|manifest| manifest.packages.contains(package_name))
            .ok_or_else(|| anyhow::format_err!("missing package `{package_name}`"))?;
        after.files.append(&mut manifest.files);
    }

    let diffs = plan(&before, &after);

    for (path, (before, after)) in diffs {
        sync(path, before, after, args.dry_run)?;
    }

    if !args.dry_run {
        if let Some(parent) = state.parent() {
            fs::create_dir_all(parent)?
        }
        fs::write(&state, serde_json::to_vec_pretty(&after)?)?;
    }

    Ok(())
}

fn load_packages<P>(root: P) -> anyhow::Result<Vec<manifest::Manifest<PathBuf>>>
where
    P: AsRef<Path>,
{
    #[derive(Deserialize)]
    struct Package {
        name: String,
        files: Vec<File>,
    }

    #[serde_with::serde_as]
    #[derive(Debug, Deserialize)]
    pub struct File {
        source: PathBuf,
        target: PathBuf,
        #[serde_as(as = "Option<misc::Octal>")]
        mode: Option<u32>,
        pre_install: Option<String>,
        post_install: Option<String>,
        pre_remove: Option<String>,
        post_remove: Option<String>,
    }

    #[tracing::instrument(err)]
    fn load(path: &Path) -> anyhow::Result<manifest::Manifest<PathBuf>> {
        let package = toml::from_str::<Package>(&fs::read_to_string(path)?)?;
        let mut manifest = manifest::Manifest::default();
        manifest.packages.insert(package.name);
        for File {
            mut source,
            mut target,
            mode,
            pre_install,
            post_install,
            pre_remove,
            post_remove,
        } in package.files
        {
            if source.is_relative() {
                source = path
                    .parent()
                    .ok_or_else(|| anyhow::format_err!("missing parent"))?
                    .join(&source);
            }
            if target.is_relative() {
                target = dirs::home_dir()
                    .ok_or_else(|| anyhow::format_err!("missing home_dir"))?
                    .join(&target);
            }
            let mode = mode.unwrap_or(0o644);
            let sha1 = misc::sha1(&source)?;
            manifest.files.insert(
                target,
                manifest::File {
                    mode,
                    sha1,
                    pre_install,
                    post_install,
                    pre_remove,
                    post_remove,
                    extra: source,
                },
            );
        }
        Ok(manifest)
    }

    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| match entry {
            Ok(entry) => {
                if entry.file_name() == concat!(env!("CARGO_BIN_NAME"), ".toml") {
                    Some(load(entry.path()))
                } else {
                    None
                }
            }
            Err(e) => Some(Err(e.into())),
        })
        .collect()
}

type Diff<'a, T, U> = (Option<&'a manifest::File<T>>, Option<&'a manifest::File<U>>);
fn plan<'a, T, U>(
    before: &'a manifest::Manifest<T>,
    after: &'a manifest::Manifest<U>,
) -> BTreeMap<&'a PathBuf, Diff<'a, T, U>> {
    let mut diffs = BTreeMap::<_, Diff<'_, _, _>>::new();
    for (path, file) in &before.files {
        diffs.entry(path).or_default().0 = Some(file);
    }
    for (path, file) in &after.files {
        diffs.entry(path).or_default().1 = Some(file);
    }
    diffs.retain(|_, (before, after)| match (before, after) {
        (Some(before), Some(after)) => (before.sha1, before.mode) != (after.sha1, after.mode),
        (None, None) => false,
        _ => true,
    });
    diffs
}

fn sync<T, U>(
    path: &Path,
    before: Option<&manifest::File<T>>,
    after: Option<&manifest::File<U>>,
    dry_run: bool,
) -> anyhow::Result<()>
where
    U: fmt::Debug + AsRef<Path>,
{
    let span = match (before.is_some(), after.is_some()) {
        (true, true) => tracing::info_span!("upgrade", ?path),
        (true, false) => tracing::info_span!("remove", ?path),
        (false, true) => tracing::info_span!("install", ?path),
        _ => unreachable!(),
    };
    let _enter = span.enter();

    if let Some(before) = before {
        let _span = tracing::info_span!("remove").entered();
        if let Some(command) = &before.pre_remove {
            exec(command, dry_run)?;
        }
        remove(&path, dry_run)?;
        if let Some(command) = &before.post_remove {
            exec(command, dry_run)?;
        }
    }
    if let Some(after) = after {
        let _span = tracing::info_span!("install").entered();
        if let Some(command) = &after.pre_install {
            exec(command, dry_run)?;
        }
        install(&after.extra, path, after.mode, dry_run)?;
        if let Some(command) = &after.post_install {
            exec(command, dry_run)?;
        }
    }

    Ok(())
}

#[tracing::instrument(err, ret)]
fn exec(command: &str, dry_run: bool) -> anyhow::Result<()> {
    if !dry_run {
        let status = Command::new("sh")
            .arg("-euxc")
            .arg(command)
            .current_dir(dirs::home_dir().ok_or_else(|| anyhow::format_err!("missing home_dir"))?)
            .status()?;
        anyhow::ensure!(status.success());
    }
    Ok(())
}

#[tracing::instrument(err, ret, skip(path))]
fn remove<P>(path: P, dry_run: bool) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    if !dry_run {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[tracing::instrument(err, ret, skip(target, mode))]
fn install<P, Q>(source: P, target: Q, mode: u32, dry_run: bool) -> anyhow::Result<()>
where
    P: fmt::Debug + AsRef<Path>,
    Q: AsRef<Path>,
{
    if !dry_run {
        if let Some(parent) = target.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, &target)?;
        fs::set_permissions(&target, Permissions::from_mode(mode))?;
    }
    Ok(())
}
