mod load;
mod misc;
mod schema;

use clap::Parser;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt;
use std::fs::{self, File};
use std::mem;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[derive(Parser)]
struct Args {
    #[clap(long, num_args = 1..)]
    install: Vec<String>,
    #[clap(long, num_args = 1..)]
    remove: Vec<String>,
    #[clap(long)]
    apply: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let data_path = dirs::data_dir()
        .ok_or_else(|| anyhow::format_err!("missing data_dir"))?
        .join(concat!(env!("CARGO_BIN_NAME"), ".json"));

    let mut before = if data_path.try_exists()? {
        serde_json::from_reader(File::open(&data_path)?)?
    } else {
        schema::State::<()>::default()
    };

    let after = {
        let mut package_names = before
            .packages
            .iter()
            .map(|package| &package.name)
            .collect::<BTreeSet<_>>();
        for package_name in &args.install {
            package_names.insert(package_name);
        }
        for package_name in &args.remove {
            package_names.remove(package_name);
        }
        tracing::info!("packages[].name" = ?package_names);

        let mut packages = load::load_packages(env::current_dir()?)?;
        let packages = package_names
            .into_iter()
            .map(|package_name| {
                packages
                    .remove(package_name)
                    .ok_or_else(|| anyhow::format_err!("missing package `{package_name}`"))
            })
            .collect::<Result<_, _>>()?;
        schema::State { packages }
    };

    let mut orphan = after
        .packages
        .iter()
        .flat_map(|package| package.files.keys())
        .map(AsRef::as_ref)
        .collect();
    sync(&mut before, &mut orphan)?;

    let diff = {
        let mut packages = BTreeMap::<_, (_, _)>::new();
        for package in &before.packages {
            packages.entry(&package.name).or_default().0 = Some(package);
        }
        for package in &after.packages {
            packages.entry(&package.name).or_default().1 = Some(package);
        }
        packages
            .into_iter()
            .filter_map(|(package_name, (before, after))| match (before, after) {
                (Some(before), Some(after)) => {
                    let before_files = before
                        .files
                        .iter()
                        .map(|(path, file)| (path, file.sha1, file.mode));
                    let after_files = after
                        .files
                        .iter()
                        .map(|(path, file)| (path, file.sha1, file.mode));
                    if before_files.eq(after_files) {
                        None
                    } else {
                        let span = tracing::info_span!("upgrade", package.name = package_name);
                        Some((span, Some(before), Some(after)))
                    }
                }
                (Some(before), None) => {
                    let span = tracing::info_span!("remove", package.name = package_name);
                    Some((span, Some(before), None))
                }
                (None, Some(after)) => {
                    let span = tracing::info_span!("install", package.name = package_name);
                    Some((span, None, Some(after)))
                }
                (None, None) => None,
            })
            .collect()
    };

    action(&diff, &orphan, args.apply)?;

    if args.apply {
        fs::write(&data_path, serde_json::to_vec_pretty(&after)?)?;
    }

    Ok(())
}

fn sync<T>(state: &mut schema::State<T>, orphan: &mut BTreeSet<&Path>) -> anyhow::Result<()> {
    for package in &mut state.packages {
        package.files = mem::take(&mut package.files)
            .into_iter()
            .filter_map(|(path, file)| {
                orphan.remove(&*path);
                Some(check(&path, file).transpose()?.map(|file| (path, file)))
            })
            .collect::<Result<_, _>>()?;
    }
    *orphan = mem::take(orphan)
        .into_iter()
        .filter_map(|path| {
            path.try_exists()
                .map(|exists| exists.then_some(path))
                .transpose()
        })
        .collect::<Result<_, _>>()?;
    Ok(())
}

#[tracing::instrument(err, skip(file))]
fn check<P, T>(path: P, mut file: schema::File<T>) -> anyhow::Result<Option<schema::File<T>>>
where
    P: AsRef<Path> + fmt::Debug,
{
    if path.as_ref().try_exists()? {
        let sha1 = misc::sha1(&path)?;
        let mode = fs::metadata(&path)?.permissions().mode();
        if sha1 != file.sha1 {
            tracing::warn!(
                actual.sha1 = hex::encode(sha1),
                expected.sha1 = hex::encode(file.sha1),
            );
        }
        if mode != file.mode {
            tracing::warn!(
                actual.mode = format!("{mode:o}"),
                expected.mode = format!("{:o}", file.mode),
            );
        }
        file.sha1 = sha1;
        file.mode = mode;
        Ok(Some(file))
    } else {
        tracing::warn!("missing");
        Ok(None)
    }
}

type Diff<'a, T, C> = (
    tracing::Span,
    Option<&'a schema::Package<T>>,
    Option<&'a schema::Package<C>>,
);

fn action<T, C>(
    diff: &Vec<Diff<'_, T, C>>,
    orphan: &BTreeSet<&Path>,
    apply: bool,
) -> anyhow::Result<()>
where
    C: AsRef<[u8]>,
{
    // pre_remove
    for (span, before, _) in diff {
        if let Some(before) = before {
            let _enter = span.enter();
            let _span = tracing::info_span!("pre_remove").entered();
            for hook in &before.hooks.pre_remove {
                misc::exec(&hook.command, apply)?;
            }
        }
    }
    // remove
    for (span, before, _) in diff {
        if let Some(before) = before {
            let _enter = span.enter();
            for path in before.files.keys() {
                misc::remove(path, apply)?;
            }
        }
    }
    {
        for path in orphan {
            let _span = tracing::info_span!("remove", ?path).entered();
            tracing::warn!("orphan");
            misc::remove(path, apply)?;
        }
    }
    // post_remove
    for (span, before, _) in diff {
        if let Some(before) = before {
            let _enter = span.enter();
            let _span = tracing::info_span!("post_remove").entered();
            for hook in &before.hooks.post_remove {
                misc::exec(&hook.command, apply)?;
            }
        }
    }

    // pre_install
    for (span, _, after) in diff {
        if let Some(after) = after {
            let _enter = span.enter();
            let _span = tracing::info_span!("pre_install").entered();
            for hook in &after.hooks.pre_install {
                misc::exec(&hook.command, apply)?;
            }
        }
    }
    // install
    for (span, _, after) in diff {
        if let Some(after) = after {
            let _enter = span.enter();
            for (path, file) in &after.files {
                misc::install(path, &file.extra, file.mode, apply)?;
            }
        }
    }
    // post_install
    for (span, _, after) in diff {
        if let Some(after) = after {
            let _enter = span.enter();
            let _span = tracing::info_span!("post_install").entered();
            for hook in &after.hooks.post_install {
                misc::exec(&hook.command, apply)?;
            }
        }
    }

    Ok(())
}
