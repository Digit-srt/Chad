use super::{context, write_runtime};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub const DEFAULT_ICON: &[u8] = include_bytes!("../assets/chad.png");
pub const ICON_EXTENSION: &str = "png";
pub const ICON_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

pub fn directory_name(name: &str) -> String {
    name.to_owned()
}
pub fn validate_output(_: &Path) -> Result<(), String> {
    Ok(())
}
pub fn files_directory(output: &Path) -> PathBuf {
    output.to_owned()
}

pub fn finish(output: &Path, name: &str, icon: Option<&[u8]>) -> Result<(), String> {
    let executable = output.join("Chad");
    write_runtime(&executable)?;
    let icon_line = if let Some(icon) = icon {
        let icon_path = output.join("AppIcon.png");
        fs::write(&icon_path, icon).map_err(|error| context(&icon_path, error))?;
        format!("Icon={}\n", desktop_value(desktop_path(&icon_path)?))
    } else {
        String::new()
    };

    let path = desktop_path(&executable)?;
    // Exec first applies desktop string escaping and then parses a quoted command argument.
    let mut argument = String::new();
    for ch in path.chars() {
        match ch {
            '\\' | '"' | '$' | '`' => {
                argument.push('\\');
                argument.push(ch);
            }
            _ => argument.push(ch),
        }
    }
    let exec = desktop_value(&argument);
    let shortcut = output.join(format!("{name}.desktop"));
    let content = format!(
        "[Desktop Entry]\nVersion=1.0\nType=Application\nName={name}\nExec=\"{exec}\"\n{icon_line}Terminal=false\n"
    );
    fs::write(&shortcut, content).map_err(|error| context(&shortcut, error))?;
    fs::set_permissions(&shortcut, fs::Permissions::from_mode(0o755))
        .map_err(|error| context(&shortcut, error))
}

fn desktop_path(path: &Path) -> Result<&str, String> {
    let path = path.to_str().ok_or("Linux kısayol yolu UTF-8 olmalı")?;
    if path.contains(['\n', '\r', '=', '%']) {
        return Err("Linux kısayol yolu satır sonu, = veya % içeremez".into());
    }
    Ok(path)
}

fn desktop_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace(' ', "\\s")
}
