use super::{context, write_runtime};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub const DEFAULT_ICON: &[u8] = include_bytes!("../assets/chad.icns");
pub const ICON_EXTENSION: &str = "icns";
pub const ICON_MAGIC: &[u8] = b"icns";

pub fn directory_name(name: &str) -> String {
    format!("{name}.app")
}

pub fn validate_output(output: &Path) -> Result<(), String> {
    if output.extension().and_then(|extension| extension.to_str()) != Some("app") {
        return Err("macOS paket yolu .app uzantılı olmalı".into());
    }
    Ok(())
}

pub fn files_directory(output: &Path) -> PathBuf {
    output.join("Contents/Resources")
}

pub fn finish(output: &Path, name: &str, icon: Option<&[u8]>) -> Result<(), String> {
    let contents = output.join("Contents");
    let executables = contents.join("MacOS");
    fs::create_dir(&executables).map_err(|error| context(&executables, error))?;
    write_runtime(&executables.join("Chad"))?;
    let icon_key = if let Some(icon) = icon {
        let icon_path = files_directory(output).join("AppIcon.icns");
        fs::write(&icon_path, icon).map_err(|error| context(&icon_path, error))?;
        "<key>CFBundleIconFile</key><string>AppIcon.icns</string>"
    } else {
        ""
    };
    let id = name.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    });
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>{name}</string>
  <key>CFBundleDisplayName</key><string>{name}</string>
  <key>CFBundleExecutable</key><string>Chad</string>
  <key>CFBundleIdentifier</key><string>org.chad.app-{id:016x}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  {icon_key}
</dict></plist>
"#
    );
    let info = contents.join("Info.plist");
    fs::write(&info, plist).map_err(|error| context(&info, error))?;
    let signed = Command::new("/usr/bin/codesign")
        .args(["--force", "--sign", "-"])
        .arg(output)
        .output()
        .map_err(|error| format!("codesign: {error}"))?;
    if !signed.status.success() {
        return Err(format!(
            "codesign: {}",
            String::from_utf8_lossy(&signed.stderr).trim()
        ));
    }
    Ok(())
}
