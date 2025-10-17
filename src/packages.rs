use crate::schema;
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::path::Path;

type Package = schema::Package<Vec<u8>>;

pub fn packages() -> BTreeMap<&'static str, fn(&mut Package) -> anyhow::Result<()>> {
    [
        ("atuin", atuin as _),
        ("base", base as _),
        ("cargo", cargo as _),
        ("emacs", emacs as _),
        ("fcitx5", fcitx5 as _),
        ("firefox", firefox as _),
        ("ghq", ghq as _),
        ("google-cloud-cli", google_cloud_cli as _),
        ("npm", npm as _),
        ("paru", paru as _),
        ("podman", podman as _),
        ("slack", slack as _),
        ("ssh", ssh as _),
        ("starship", starship as _),
        ("sway", sway as _),
        ("tmux", tmux as _),
        ("uv", uv as _),
    ]
    .into_iter()
    .collect()
}

macro_rules! template {
    ($path:literal) => {{
        #[derive(::askama::Template)]
        #[template(escape = "none", path = $path)]
        #[allow(dead_code)]
        struct Template {
            env: ::std::collections::BTreeMap<String, String>,
            uid: u32,
        }

        let template = Template {
            env: ::std::env::vars().collect(),
            uid: ::nix::unistd::getuid().as_raw(),
        };
        ::askama::Template::render(&template).map(|mut s| {
            if !s.ends_with('\n') {
                s.push('\n');
            }
            s
        })
    }};
}

// atuin
// bash-preexec
fn atuin(package: &mut Package) -> anyhow::Result<()> {
    package.file(
        // https://github.com/atuinsh/atuin/issues/2738#issuecomment-2876082481
        ".bashrc.d/60-atuin.bash",
        include_str!("packages/atuin/atuin.bash"),
        None,
    )?;
    package.file(
        ".config/atuin/config.toml",
        include_str!("packages/atuin/config.toml"),
        None,
    )?;

    Ok(())
}

// xdg-user-dirs
fn base(package: &mut Package) -> anyhow::Result<()> {
    package.file(".bashrc", include_str!("packages/base/bashrc.bash"), None)?;
    package.file(
        ".config/user-dirs.dirs",
        include_str!("packages/base/user-dirs.dirs"),
        None,
    )?;
    package.pre_install(["mkdir", "-p", "downloads"]);
    Ok(())
}

// rustup
fn cargo(package: &mut Package) -> anyhow::Result<()> {
    package.file(
        ".bashrc.d/50-cargo.bash",
        include_str!("packages/cargo/cargo.bash"),
        None,
    )?;
    package.file(
        ".cargo/config.toml",
        template!("packages/cargo/config.toml")?,
        None,
    )?;
    Ok(())
}

// emacs
fn emacs(package: &mut Package) -> anyhow::Result<()> {
    let mut desktop = ini::Ini::load_from_file("/usr/share/applications/emacs.desktop")?;
    for (_, properties) in &mut desktop {
        for (k, v) in properties {
            if k == "Exec" {
                v.insert_str(
                    0,
                    "/usr/bin/systemd-run --user --quiet --scope --slice=emacs.slice ",
                );
            }
        }
    }
    let mut s = Vec::new();
    desktop.write_to(&mut s)?;
    package.file(".local/share/applications/emacs.desktop", s, None)?;
    Ok(())
}

// fcitx5-gtk
// fcitx5-qt
// fcitx5-skk
fn fcitx5(package: &mut Package) -> anyhow::Result<()> {
    package.file(
        ".bashrc.d/50-fcitx5.bash",
        include_str!("packages/fcitx5/fcitx5.bash"),
        None,
    )?;
    package.file(
        ".config/fcitx5/conf/skk.conf",
        include_str!("packages/fcitx5/skk.conf"),
        None,
    )?;
    package.file(
        ".config/fcitx5/config",
        include_str!("packages/fcitx5/config"),
        None,
    )?;
    package.file(
        ".config/fcitx5/profile",
        include_str!("packages/fcitx5/profile"),
        Some(0o100600),
    )?;
    package.file(
        ".config/systemd/user/fcitx5.service",
        include_str!("packages/fcitx5/fcitx5.service"),
        None,
    )?;
    package.file(
        ".local/share/fcitx5/skk/dictionary_list",
        include_str!("packages/fcitx5/dictionary_list"),
        None,
    )?;
    package.post_install(["systemctl", "--user", "daemon-reload"]);
    package.post_install(["systemctl", "--user", "enable", "fcitx5.service"]);
    package.pre_remove(["systemctl", "--user", "disable", "fcitx5.service"]);
    Ok(())
}

// firefox
fn firefox(package: &mut Package) -> anyhow::Result<()> {
    package.file(
        ".bashrc.d/50-firefox.bash",
        include_str!("packages/firefox/firefox.bash"),
        None,
    )?;
    let mut desktop = ini::Ini::load_from_file("/usr/share/applications/firefox.desktop")?;
    for (_, properties) in &mut desktop {
        for (k, v) in properties {
            if k == "Exec" {
                v.insert_str(
                    0,
                    "/usr/bin/systemd-run --user --quiet --scope --slice=firefox.slice ",
                );
            }
        }
    }
    let mut s = Vec::new();
    desktop.write_to(&mut s)?;
    package.file(".local/share/applications/firefox.desktop", s, None)?;
    Ok(())
}

// ghq
// skim
fn ghq(package: &mut Package) -> anyhow::Result<()> {
    package.file(
        ".bashrc.d/50-ghq.bash",
        include_str!("packages/ghq/ghq.bash"),
        None,
    )?;
    Ok(())
}

// podman
fn google_cloud_cli(package: &mut Package) -> anyhow::Result<()> {
    package.file(
        ".bashrc.d/50-google-cloud-cli.bash",
        include_str!("packages/google-cloud-cli/google-cloud-cli.bash"),
        None,
    )?;
    package.file(
        ".config/containers/systemd/google-cloud-cli.container",
        include_str!("packages/google-cloud-cli/google-cloud-cli.container"),
        None,
    )?;
    package.file(
        ".local/bin/gcloud",
        include_str!("packages/google-cloud-cli/google-cloud-cli"),
        Some(0o100755),
    )?;
    package.file(
        ".local/bin/docker-credential-gcloud",
        include_str!("packages/google-cloud-cli/google-cloud-cli"),
        Some(0o100755),
    )?;
    package.pre_install(["mkdir", "-p", ".config/gcloud"]);
    package.post_install(["systemctl", "--user", "daemon-reload"]);
    Ok(())
}

// npm
fn npm(package: &mut Package) -> anyhow::Result<()> {
    package.file(".npmrc", template!("packages/npm/npmrc")?, None)?;
    Ok(())
}

// paru
fn paru(package: &mut Package) -> anyhow::Result<()> {
    package.file(
        ".config/paru/paru.conf",
        template!("packages/paru/paru.conf")?,
        None,
    )?;
    Ok(())
}

// podman
fn podman(package: &mut Package) -> anyhow::Result<()> {
    package.file(
        ".bashrc.d/50-podman.bash",
        include_str!("packages/podman/podman.bash"),
        None,
    )?;
    package.file(
        ".config/containers/containers.conf",
        include_str!("packages/podman/containers.conf"),
        None,
    )?;
    package.file(
        ".config/containers/storage.conf",
        template!("packages/podman/storage.conf")?,
        None,
    )?;
    Ok(())
}

// slack
fn slack(package: &mut Package) -> anyhow::Result<()> {
    let mut desktop = ini::Ini::load_from_file("/usr/share/applications/slack.desktop")?;
    for (_, properties) in &mut desktop {
        for (k, v) in properties {
            if k == "Exec" {
                v.insert_str(
                    0,
                    "/usr/bin/systemd-run --user --quiet --scope --slice=slack.slice ",
                );
            }
        }
    }
    let mut s = Vec::new();
    desktop.write_to(&mut s)?;
    package.file(".local/share/applications/slack.desktop", s, None)?;
    Ok(())
}

// openssh
fn ssh(package: &mut Package) -> anyhow::Result<()> {
    package.file(
        ".bashrc.d/50-ssh-agent.bash",
        include_str!("packages/ssh/ssh-agent.bash"),
        None,
    )?;
    package.file(".ssh/config", template!("packages/ssh/config")?, None)?;
    Ok(())
}

// starship
fn starship(package: &mut Package) -> anyhow::Result<()> {
    package.file(
        ".bashrc.d/50-starship.bash",
        include_str!("packages/starship/starship.bash"),
        None,
    )?;
    Ok(())
}

// foot
// fuzzel
// i3status-rust
// noto-fonts
// noto-fonts-cjk
// noto-fonts-emoji
// otf-font-awesome
// sway
// swaybg
// swayidle
// swaylock
// ttf-iosevka-nerd
// xorg-xwayland
fn sway(package: &mut Package) -> anyhow::Result<()> {
    package.file(
        ".bashrc.d/50-sway.bash",
        include_str!("packages/sway/sway.bash"),
        None,
    )?;
    package.file(
        ".config/foot/foot.ini",
        include_str!("packages/sway/foot.ini"),
        None,
    )?;
    package.file(
        ".config/fuzzel/fuzzel.ini",
        include_str!("packages/sway/fuzzel.ini"),
        None,
    )?;
    package.file(
        ".config/i3status-rust/config.toml",
        include_str!("packages/sway/i3status-rust.toml"),
        None,
    )?;
    package.file(
        ".config/sway/config",
        include_str!("packages/sway/config"),
        None,
    )?;
    package.file(
        ".config/systemd/user/swayidle.service",
        include_str!("packages/sway/swayidle.service"),
        None,
    )?;
    package.file(
        ".config/systemd/user/sway-session.target",
        include_str!("packages/sway/sway-session.target"),
        None,
    )?;
    package.file(
        ".xkb/symbols/us_henkan",
        include_str!("packages/sway/us_henkan"),
        None,
    )?;
    package.post_install(["systemctl", "--user", "daemon-reload"]);
    package.post_install(["systemctl", "--user", "enable", "swayidle.service"]);
    package.pre_remove(["systemctl", "--user", "disable", "swayidle.service"]);
    Ok(())
}

// tmux
fn tmux(package: &mut Package) -> anyhow::Result<()> {
    package.file(
        ".config/tmux/tmux.conf",
        include_str!("packages/tmux/tmux.conf"),
        None,
    )?;
    Ok(())
}

// uv
fn uv(package: &mut Package) -> anyhow::Result<()> {
    package.file(
        ".config/uv/uv.toml",
        template!("packages/uv/uv.toml")?,
        None,
    )?;
    Ok(())
}

trait PackageExt {
    fn file<P, C>(&mut self, path: P, content: C, mode: Option<u32>) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
        C: AsRef<[u8]>;
    fn pre_install<I>(&mut self, command: I)
    where
        I: IntoIterator,
        I::Item: Into<String>;
    fn post_install<I>(&mut self, command: I)
    where
        I: IntoIterator,
        I::Item: Into<String>;
    fn pre_remove<I>(&mut self, command: I)
    where
        I: IntoIterator,
        I::Item: Into<String>;
}

impl PackageExt for Package {
    fn file<P, C>(&mut self, path: P, content: C, mode: Option<u32>) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
        C: AsRef<[u8]>,
    {
        let path = if path.as_ref().is_relative() {
            dirs::home_dir()
                .ok_or_else(|| anyhow::format_err!("missing home_dir"))?
                .join(path)
        } else {
            path.as_ref().to_path_buf()
        };
        let sha1 = Sha1::digest(content.as_ref()).into();
        let mode = mode.unwrap_or(0o100644);
        self.files.insert(
            path,
            schema::File {
                sha1,
                mode,
                extra: content.as_ref().to_vec(),
            },
        );
        Ok(())
    }

    fn pre_install<I>(&mut self, command: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.hooks.pre_install.push(schema::Hook {
            command: command.into_iter().map(Into::into).collect(),
        });
    }

    fn post_install<I>(&mut self, command: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.hooks.post_install.push(schema::Hook {
            command: command.into_iter().map(Into::into).collect(),
        });
    }

    fn pre_remove<I>(&mut self, command: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.hooks.pre_remove.push(schema::Hook {
            command: command.into_iter().map(Into::into).collect(),
        });
    }
}
