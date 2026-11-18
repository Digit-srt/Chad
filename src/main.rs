#![allow(non_snake_case)]

mod android;
mod package;

use std::{env, ffi::OsString, path::PathBuf, process::ExitCode};

const HELP: &str = "Chad -build <giriş.html> [--name <uygulama>] [--out <paket-yolu>] [--icon <dosya> | --no-icon]\n\
Varsayılan hedef masaüstünde çalıştırdığın işletim sistemidir.\n\
Android: --target android --site <https://alan-adı/dizin/> --app-id <com.ornek.uygulama> [--project-only]\n\
İkon biçimi: Windows .ico, macOS .icns, Linux ve Android .png.";

enum Icon {
    Default,
    Custom(PathBuf),
    None,
}

struct Build {
    entry: PathBuf,
    name: Option<String>,
    output: Option<PathBuf>,
    icon: Icon,
    android: Option<android::Options>,
}

fn main() -> ExitCode {
    match run(env::args_os().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Chad: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), String> {
    let command = args.next();
    let command = command.as_ref().map(|arg| arg.to_string_lossy());
    match command.as_deref() {
        None | Some("--help" | "-h" | "help") => println!("{HELP}"),
        Some("--version" | "-V") => println!("Chad {}", env!("CARGO_PKG_VERSION")),
        Some("-build" | "build" | "--build") => {
            let build = parse_build(args)?;
            let output = package::build(build)?;
            println!("Chad paketi: {}", output.display());
        }
        Some(command) => return Err(format!("bilinmeyen komut: {command}\n{HELP}")),
    }
    Ok(())
}

fn parse_build(mut args: impl Iterator<Item = OsString>) -> Result<Build, String> {
    let entry = args.next().ok_or_else(|| HELP.to_owned())?;
    let mut build = Build {
        entry: entry.into(),
        name: None,
        output: None,
        icon: Icon::Default,
        android: None,
    };
    let mut target = None;
    let mut site = None;
    let mut app_id = None;
    let mut project_only = false;
    while let Some(flag) = args.next() {
        let mut value = || {
            args.next()
                .ok_or_else(|| format!("{} için değer gerekli", flag.to_string_lossy()))
        };
        match flag.to_str() {
            Some("--name") if build.name.is_none() => {
                build.name = Some(
                    value()?
                        .into_string()
                        .map_err(|_| "uygulama adı UTF-8 olmalı")?,
                );
            }
            Some("--out") if build.output.is_none() => build.output = Some(value()?.into()),
            Some("--icon") if matches!(build.icon, Icon::Default) => {
                build.icon = Icon::Custom(value()?.into());
            }
            Some("--no-icon") if matches!(build.icon, Icon::Default) => build.icon = Icon::None,
            Some("--target") if target.is_none() => {
                target = Some(value()?.into_string().map_err(|_| "geçersiz hedef")?);
            }
            Some("--site") if site.is_none() => {
                site = Some(value()?.into_string().map_err(|_| "geçersiz site adresi")?);
            }
            Some("--app-id") if app_id.is_none() => {
                app_id = Some(
                    value()?
                        .into_string()
                        .map_err(|_| "geçersiz Android kimliği")?,
                );
            }
            Some("--project-only") if !project_only => project_only = true,
            _ => {
                return Err(format!(
                    "geçersiz veya tekrarlanan seçenek: {}",
                    flag.to_string_lossy()
                ));
            }
        }
    }
    match target.as_deref() {
        Some("android") => {
            build.android = Some(android::Options::new(
                &site.ok_or("Android için --site gerekli")?,
                app_id.ok_or("Android için --app-id gerekli")?,
                project_only,
            )?);
        }
        None | Some("desktop") if site.is_none() && app_id.is_none() && !project_only => {}
        None | Some("desktop") => {
            return Err("--site, --app-id ve --project-only Android hedefi içindir".into());
        }
        Some(target) => return Err(format!("geçersiz hedef: {target}")),
    }
    Ok(build)
}
