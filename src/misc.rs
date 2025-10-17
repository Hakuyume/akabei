use sha1::{Digest, Sha1};
use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, File, Permissions};
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

serde_with::serde_conv!(pub Octal, u32, |value| format!("{value:o}"), |value: String| {
    u32::from_str_radix(&value, 8)
});

#[tracing::instrument(err)]
pub fn sha1<P>(path: P) -> io::Result<[u8; 20]>
where
    P: AsRef<Path> + fmt::Debug,
{
    let mut hasher = Sha1::new();
    io::copy(&mut File::open(path)?, &mut hasher)?;
    Ok(hasher.finalize().into())
}

#[tracing::instrument(err, ret)]
pub fn remove<P>(path: P, apply: bool) -> io::Result<()>
where
    P: AsRef<Path> + fmt::Debug,
{
    if apply {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[tracing::instrument(err, fields(mode = format!("{mode:o}")), ret, skip(content))]
pub fn install<P, C>(path: P, content: C, mode: u32, apply: bool) -> io::Result<()>
where
    P: AsRef<Path> + fmt::Debug,
    C: AsRef<[u8]>,
{
    if apply {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
        fs::set_permissions(&path, Permissions::from_mode(mode))?;
    }
    Ok(())
}

#[tracing::instrument(err, ret)]
pub fn exec<I>(command: I, apply: bool) -> anyhow::Result<()>
where
    I: IntoIterator + fmt::Debug,
    I::Item: AsRef<OsStr>,
{
    let mut command = command.into_iter();
    if apply && let Some(program) = command.next() {
        let status = Command::new(program)
            .args(command)
            .current_dir(dirs::home_dir().ok_or_else(|| anyhow::format_err!("missing home_dir"))?)
            .status()?;
        anyhow::ensure!(status.success());
    }
    Ok(())
}
