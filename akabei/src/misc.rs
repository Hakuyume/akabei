use sha1::{Digest, Sha1};
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

#[tracing::instrument(err, fields(mode = format!("{mode:o}")), ret)]
pub fn install<P, Q>(source: P, target: Q, mode: u32, apply: bool) -> io::Result<()>
where
    P: AsRef<Path> + fmt::Debug,
    Q: AsRef<Path> + fmt::Debug,
{
    if apply {
        if let Some(parent) = target.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, &target)?;
        fs::set_permissions(&target, Permissions::from_mode(mode))?;
    }
    Ok(())
}

#[tracing::instrument(err, ret)]
pub fn exec(command: &str, apply: bool) -> anyhow::Result<()> {
    if apply {
        let status = Command::new("sh")
            .arg("-euxc")
            .arg(command)
            .current_dir(dirs::home_dir().ok_or_else(|| anyhow::format_err!("missing home_dir"))?)
            .status()?;
        anyhow::ensure!(status.success());
    }
    Ok(())
}
