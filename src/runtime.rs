#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    match launch() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report_error(&error);
            ExitCode::FAILURE
        }
    }
}

fn launch() -> Result<(), String> {
    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let folder = executable
        .parent()
        .ok_or("Chad paket klasörü bulunamadı.")?;
    #[cfg(target_os = "macos")]
    let resources = folder
        .parent()
        .ok_or("Chad uygulama paketi bozuk.")?
        .join("Resources");
    #[cfg(not(target_os = "macos"))]
    let resources = folder.to_owned();

    let entry = fs::read_to_string(resources.join("chad-entry"))
        .map_err(|error| format!("Chad giriş dosyası okunamadı. {error}"))?;
    let entry_name = Path::new(&entry);
    if entry_name.file_name() != Some(entry_name.as_os_str()) {
        return Err("Chad giriş dosyasının adı geçersiz.".into());
    }
    let file = resources.join("app").join(entry_name);
    if !file.is_file() {
        return Err(format!("Uygulama dosyası bulunamadı. {}", file.display()));
    }
    let chrome = find_chrome().ok_or(
        "Bu uygulama Google Chrome gerektiriyor. Google Chrome’u kurup uygulamayı tekrar aç.",
    )?;
    let url = file_url(&file)?;
    Command::new(&chrome)
        .arg(format!("--app={url}"))
        .spawn()
        .map_err(|error| format!("Google Chrome açılamadı. {error}"))?;
    Ok(())
}

fn file_url(path: &Path) -> Result<String, String> {
    use std::fmt::Write;
    let raw = path.to_str().ok_or("Uygulama yolu UTF-8 olmalı.")?;
    #[cfg(windows)]
    let raw = raw.replace('\\', "/");
    #[cfg(windows)]
    let (prefix, raw) = if let Some(share) = raw.strip_prefix("//?/UNC/").or_else(|| {
        raw.strip_prefix("//")
            .filter(|value| !value.starts_with("?/"))
    }) {
        ("file://", share)
    } else {
        ("file:///", raw.strip_prefix("//?/").unwrap_or(&raw))
    };
    #[cfg(not(windows))]
    let (prefix, raw) = ("file://", raw);

    let mut url = prefix.to_owned();
    for (index, byte) in raw.bytes().enumerate() {
        if byte.is_ascii_alphanumeric()
            || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~')
            || (cfg!(windows) && index == 1 && byte == b':')
        {
            url.push(char::from(byte));
        } else {
            write!(url, "%{byte:02X}").unwrap();
        }
    }
    Ok(url)
}

#[cfg(target_os = "windows")]
fn find_chrome() -> Option<PathBuf> {
    [
        "ProgramW6432",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "LOCALAPPDATA",
    ]
    .into_iter()
    .filter_map(env::var_os)
    .map(|base| PathBuf::from(base).join("Google/Chrome/Application/chrome.exe"))
    .find(|path| path.is_absolute() && path.is_file())
}

#[cfg(target_os = "macos")]
fn find_chrome() -> Option<PathBuf> {
    let relative = Path::new("Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
    let system = Path::new("/").join(relative);
    let user = env::var_os("HOME").map(|home| PathBuf::from(home).join(relative));
    std::iter::once(system)
        .chain(user)
        .find(|path| path.is_absolute() && path.is_file())
}

#[cfg(target_os = "linux")]
fn find_chrome() -> Option<PathBuf> {
    let names = ["google-chrome-stable", "google-chrome"];
    let search_path = env::var_os("PATH").unwrap_or_default();
    ["/usr/bin", "/usr/local/bin", "/opt/google/chrome"]
        .into_iter()
        .map(PathBuf::from)
        .chain(env::split_paths(&search_path))
        .filter(|directory| directory.is_absolute())
        .flat_map(|directory| names.map(|name| directory.join(name)))
        .find(|path| {
            use std::os::unix::fs::PermissionsExt;
            path.metadata().is_ok_and(|metadata| {
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            })
        })
}

#[cfg(target_os = "windows")]
fn report_error(message: &str) {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(
            owner: *mut std::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            style: u32,
        ) -> i32;
    }
    let message: Vec<u16> = message.encode_utf16().chain([0]).collect();
    let title: Vec<u16> = "Chad".encode_utf16().chain([0]).collect();
    // This runtime uses the GUI subsystem, so report startup errors in a dialog.
    unsafe { MessageBoxW(std::ptr::null_mut(), message.as_ptr(), title.as_ptr(), 0x10) };
}

#[cfg(target_os = "macos")]
fn report_error(message: &str) {
    let script =
        "on run argv\n display alert \"Chad\" message (item 1 of argv) as critical\nend run";
    if Command::new("/usr/bin/osascript")
        .args(["-e", script, message])
        .status()
        .is_err()
    {
        eprintln!("Chad: {message}");
    }
}

#[cfg(target_os = "linux")]
fn report_error(message: &str) {
    eprintln!("Chad: {message}");
    for (program, arguments) in [
        ("zenity", vec!["--error", "--title=Chad", "--text", message]),
        ("kdialog", vec!["--error", message, "--title", "Chad"]),
    ] {
        if Command::new(program).args(arguments).spawn().is_ok() {
            return;
        }
    }
}
