use sha1::{Digest, Sha1};
use std::fs::File;
use std::io;
use std::path::Path;

serde_with::serde_conv!(pub Octal, u32, |value| format!("{value:o}"), |value: String| {
    u32::from_str_radix(&value, 8)
});

pub fn sha1<P>(path: P) -> io::Result<[u8; 20]>
where
    P: AsRef<Path>,
{
    let mut hasher = Sha1::new();
    io::copy(&mut File::open(path)?, &mut hasher)?;
    Ok(hasher.finalize().into())
}
