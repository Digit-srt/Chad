use crate::{Build, Icon, android};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

const RUNTIME: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/chad-runtime.bin"));

#[cfg(target_os = "windows")]
#[path = "windows_package.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "macos_package.rs"]
mod platform;
#[cfg(target_os = "linux")]
#[path = "linux_package.rs"]
mod platform;

pub fn build(config: Build) -> Result<PathBuf, String> {
    let entry = config
        .entry
        .canonicalize()
        .map_err(|error| context(&config.entry, error))?;
    let extension = entry
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if !entry.is_file()
        || !(extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm"))
    {
        return Err("giriş var olan bir .html veya .htm dosyası olmalı".into());
    }
    let entry_name = entry
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("giriş dosyasının adı UTF-8 olmalı")?;
    if entry_name.starts_with('.') {
        return Err("giriş dosyasının adı nokta ile başlayamaz".into());
    }
    let name = match config.name {
        Some(name) => {
            validate_name(&name)?;
            name
        }
        None => {
            let stem = entry
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default();
            let cleaned: String = stem
                .trim()
                .chars()
                .map(|ch| {
                    if ch.is_alphanumeric() || matches!(ch, ' ' | '-' | '_') {
                        ch
                    } else {
                        '_'
                    }
                })
                .collect();
            if validate_name(&cleaned).is_ok() {
                cleaned
            } else {
                "ChadApp".into()
            }
        }
    };
    let (default_icon, icon_extension, icon_magic) = if config.android.is_some() {
        (android::DEFAULT_ICON, "png", android::PNG_MAGIC)
    } else {
        (
            platform::DEFAULT_ICON,
            platform::ICON_EXTENSION,
            platform::ICON_MAGIC,
        )
    };
    let icon = load_icon(config.icon, default_icon, icon_extension, icon_magic)?;
    let output = config.output.unwrap_or_else(|| {
        let directory = if config.android.is_some() {
            format!("{name}-android")
        } else {
            platform::directory_name(&name)
        };
        Path::new("dist").join(directory)
    });
    if config.android.is_none() {
        platform::validate_output(&output)?;
    }
    let folder_name = output.file_name().ok_or("geçersiz paket yolu")?;
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|error| context(parent, error))?;
    let output = parent
        .canonicalize()
        .map_err(|error| context(parent, error))?
        .join(folder_name);
    fs::create_dir(&output).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            return format!(
                "paket yolu zaten var: {}. Yeni bir --out yolu seç.",
                output.display()
            );
        }
        context(&output, error)
    })?;

    let source = entry.parent().expect("a canonical file has a parent");
    let result = match config.android {
        Some(options) => {
            android::build(&output, &name, source, entry_name, icon.as_deref(), options)
        }
        None => create_package(&output, &name, source, entry_name, icon.as_deref()),
    };
    if let Err(error) = result {
        return match fs::remove_dir_all(&output) {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(format!(
                "{error}\nYarım kalan paket silinemedi: {}",
                context(&output, cleanup_error)
            )),
        };
    }
    Ok(output)
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 100
        || name.trim() != name
        || !name
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, ' ' | '-' | '_'))
        || !name.chars().any(char::is_alphanumeric)
    {
        return Err(
            "uygulama adı harf, sayı, boşluk, tire veya alt çizgi içerebilir (en çok 100 bayt)"
                .into(),
        );
    }
    let ascii = name.to_ascii_uppercase();
    if matches!(ascii.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            matches!(
                ascii.strip_prefix(prefix),
                Some("1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³")
            )
        })
    {
        return Err("bu uygulama adı işletim sistemi tarafından ayrılmış".into());
    }
    Ok(())
}

fn load_icon(
    icon: Icon,
    default: &[u8],
    extension: &str,
    magic: &[u8],
) -> Result<Option<Vec<u8>>, String> {
    match icon {
        Icon::None => Ok(None),
        Icon::Default => Ok(Some(default.to_vec())),
        Icon::Custom(path) => {
            if !path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
            {
                return Err(format!("bu sistemde ikon .{} biçiminde olmalı", extension));
            }
            let bytes = fs::read(&path).map_err(|error| context(&path, error))?;
            if !bytes.starts_with(magic) {
                return Err(format!("geçersiz ikon dosyası: {}", path.display()));
            }
            Ok(Some(bytes))
        }
    }
}

fn create_package(
    output: &Path,
    name: &str,
    source: &Path,
    entry: &str,
    icon: Option<&[u8]>,
) -> Result<(), String> {
    let files = platform::files_directory(output);
    let app = files.join("app");
    stage_web_files(source, &app, output)?;
    let marker = files.join("chad-entry");
    fs::write(&marker, entry).map_err(|error| context(&marker, error))?;
    platform::finish(output, name, icon)
}

pub(crate) fn stage_web_files(source: &Path, app: &Path, output: &Path) -> Result<(), String> {
    fs::create_dir_all(app).map_err(|error| context(app, error))?;
    let cli = env::current_exe()
        .and_then(|path| path.canonicalize())
        .map_err(|error| format!("Chad CLI yolu: {error}"))?;
    copy_app(source, app, &[output, &cli])
}

fn copy_app(source: &Path, destination: &Path, excluded: &[&Path]) -> Result<(), String> {
    for item in fs::read_dir(source).map_err(|error| context(source, error))? {
        let item = item.map_err(|error| context(source, error))?;
        let path = item.path();
        let name = item.file_name();
        if excluded.contains(&path.as_path()) || name.to_string_lossy().starts_with('.') {
            continue;
        }
        let kind = item.file_type().map_err(|error| context(&path, error))?;
        let target = destination.join(&name);
        if kind.is_dir() {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
                let metadata = item.metadata().map_err(|error| context(&path, error))?;
                if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                    continue;
                }
            }
            if matches!(name.to_str(), Some("dist" | "target" | "node_modules")) {
                continue;
            }
            fs::create_dir(&target).map_err(|error| context(&target, error))?;
            copy_app(&path, &target, excluded)?;
        } else if kind.is_file() {
            fs::copy(&path, &target).map_err(|error| context(&path, error))?;
        }
    }
    Ok(())
}

pub(crate) fn context(path: &Path, error: io::Error) -> String {
    format!("{}: {error}", path.display())
}

fn write_runtime(path: &Path) -> Result<(), String> {
    fs::write(path, RUNTIME).map_err(|error| context(path, error))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .map_err(|error| context(path, error))?;
    }
    Ok(())
}
