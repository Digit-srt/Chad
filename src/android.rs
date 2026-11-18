use crate::package::{context, stage_web_files};
use std::{
    env,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub const DEFAULT_ICON: &[u8] = include_bytes!("../assets/chad-android.png");
pub const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

const SETTINGS: &str = include_str!("../android/settings.gradle");
const MANIFEST: &str = include_str!("../android/AndroidManifest.xml");
const STRINGS: &str = include_str!("../android/strings.xml");
const ACTIVITY: &str = include_str!("../android/ChadActivity.java");
const APP_GRADLE: &str = include_str!("../android/app.gradle");

pub struct Options {
    origin: String,
    directory: String,
    app_id: String,
    project_only: bool,
}

impl Options {
    pub fn new(site: &str, app_id: String, project_only: bool) -> Result<Self, String> {
        let site = site
            .strip_prefix("https://")
            .ok_or("--site adresi https:// ile başlamalı")?;
        let (host, directory) = site.split_once('/').unwrap_or((site, ""));
        let valid_host = host.contains('.')
            && host.len() <= 253
            && host.split('.').all(|part| {
                !part.is_empty()
                    && part.len() <= 63
                    && !part.starts_with('-')
                    && !part.ends_with('-')
                    && part
                        .bytes()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-')
            });
        if !valid_host || host.parse::<std::net::Ipv4Addr>().is_ok() {
            return Err("--site halka açık bir HTTPS alan adı kullanmalı".into());
        }
        let directory = directory.strip_suffix('/').unwrap_or(directory);
        if !directory.is_empty()
            && !directory.split('/').all(|segment| {
                !segment.is_empty()
                    && !segment.starts_with('.')
                    && segment.bytes().all(|ch| {
                        ch.is_ascii_alphanumeric() || matches!(ch, b'-' | b'_' | b'.' | b'~')
                    })
            })
        {
            return Err(
                "--site geçerli bir HTTPS dizin adresi olmalı (sorgu ve parça olmadan)".into(),
            );
        }
        let valid_id = app_id.contains('.')
            && app_id.split('.').all(|part| {
                part.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                    && part
                        .bytes()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_')
            });
        if !valid_id {
            return Err("--app-id com.ornek.uygulama biçiminde olmalı".into());
        }
        Ok(Self {
            origin: format!("https://{}", host.to_ascii_lowercase()),
            directory: directory.into(),
            app_id,
            project_only,
        })
    }

    fn start_url(&self, entry: &str) -> String {
        let mut url = format!("{}/", self.origin);
        if !self.directory.is_empty() {
            url.push_str(&self.directory);
            url.push('/');
        }
        for byte in entry.bytes() {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                url.push(char::from(byte));
            } else {
                write!(url, "%{byte:02X}").unwrap();
            }
        }
        url
    }
}

pub fn build(
    output: &Path,
    name: &str,
    source: &Path,
    entry: &str,
    icon: Option<&[u8]>,
    options: Options,
) -> Result<(), String> {
    let sdk = if options.project_only {
        None
    } else {
        Some(sdk_tools()?)
    };
    let web = output.join("web");
    stage_web_files(source, &web.join(&options.directory), output)?;
    let project = output.join("android");
    create_project(&project, name, entry, icon, &options)?;
    if let Some((gradle, java, apksigner)) = sdk {
        let command_dir = tool_path(&project);
        let status = Command::new(gradle)
            .current_dir(&command_dir)
            .args(["--no-daemon", ":app:assembleDebug"])
            .status()
            .map_err(|error| format!("Gradle başlatılamadı: {error}"))?;
        if !status.success() {
            return Err(format!("Android derlemesi başarısız oldu ({status})"));
        }
        let built_apk = project.join("app/build/outputs/apk/debug/app-debug.apk");
        let apk_name = format!("{name}.apk");
        let cert = Command::new(java)
            .current_dir(&command_dir)
            .arg("-jar")
            .arg(tool_path(&apksigner))
            .args([
                "verify",
                "--print-certs",
                "app/build/outputs/apk/debug/app-debug.apk",
            ])
            .output()
            .map_err(|error| format!("apksigner başlatılamadı: {error}"))?;
        if !cert.status.success() {
            return Err(format!(
                "APK imzası doğrulanamadı: {}",
                String::from_utf8_lossy(&cert.stderr).trim()
            ));
        }
        let fingerprint = certificate_fingerprint(&String::from_utf8_lossy(&cert.stdout))?;
        let asset_links = format!(
            r#"[
  {{
    "relation": ["delegate_permission/common.handle_all_urls"],
    "target": {{
      "namespace": "android_app",
      "package_name": "{}",
      "sha256_cert_fingerprints": ["{fingerprint}"]
    }}
  }}
]
"#,
            options.app_id
        );
        save(&web, ".well-known/assetlinks.json", asset_links)?;
        let apk = output.join(apk_name);
        fs::copy(&built_apk, &apk).map_err(|error| context(&apk, error))?;
        fs::remove_dir_all(&project).map_err(|error| context(&project, error))?;
    }
    Ok(())
}

fn create_project(
    project: &Path,
    name: &str,
    entry: &str,
    icon: Option<&[u8]>,
    options: &Options,
) -> Result<(), String> {
    save(project, "settings.gradle", SETTINGS)?;
    save(project, "gradle.properties", "android.useAndroidX=true\n")?;
    save(
        project,
        "app/build.gradle",
        APP_GRADLE.replace("@APP_ID@", &options.app_id),
    )?;
    let icon_attribute = if let Some(icon) = icon {
        save(
            project,
            "app/src/main/res/drawable-nodpi/chad_icon.png",
            icon,
        )?;
        "android:icon=\"@drawable/chad_icon\""
    } else {
        ""
    };
    save(
        project,
        "app/src/main/AndroidManifest.xml",
        MANIFEST.replace("@ICON@", icon_attribute),
    )?;
    let strings = STRINGS
        .replace("@NAME@", name)
        .replace("@ORIGIN@", &options.origin)
        .replace("@START_URL@", &options.start_url(entry));
    save(project, "app/src/main/res/values/strings.xml", strings)?;
    save(
        project,
        "app/src/main/java/org/chad/runtime/ChadActivity.java",
        ACTIVITY,
    )
}

fn save(root: &Path, relative: &str, data: impl AsRef<[u8]>) -> Result<(), String> {
    let path = root.join(relative);
    let parent = path
        .parent()
        .expect("generated file has a parent directory");
    fs::create_dir_all(parent).map_err(|error| context(parent, error))?;
    fs::write(&path, data).map_err(|error| context(&path, error))
}

fn sdk_tools() -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let sdk = env::var_os("ANDROID_HOME")
        .or_else(|| env::var_os("ANDROID_SDK_ROOT"))
        .map(PathBuf::from)
        .ok_or("Android SDK yolunu ANDROID_HOME ile belirt veya --project-only kullan")?;
    let sdk = sdk.canonicalize().map_err(|error| context(&sdk, error))?;
    let signer = sdk.join("build-tools/35.0.0/lib/apksigner.jar");
    if !sdk.join("platforms/android-35/android.jar").is_file() || !signer.is_file() {
        return Err("Android SDK API 35 ve Android Build Tools 35.0.0 gerekli".into());
    }
    let gradle = on_path(if cfg!(windows) {
        "gradle.bat"
    } else {
        "gradle"
    })
    .ok_or("Gradle 8.11.1 PATH üzerinde bulunamadı")?;
    let java_file = if cfg!(windows) { "java.exe" } else { "java" };
    let java = env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin").join(java_file))
        .filter(|path| path.is_absolute() && path.is_file())
        .or_else(|| on_path(java_file))
        .ok_or("Java 17 bulunamadı. JAVA_HOME veya PATH ayarını kontrol et")?;
    Ok((gradle, java, signer))
}

fn on_path(file: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .filter(|path| path.is_absolute())
        .map(|path| path.join(file))
        .find(|path| path.is_file())
}

fn tool_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    if let Some(drive_path) = path.to_str().and_then(|path| path.strip_prefix(r"\\?\")) {
        if drive_path.as_bytes().get(1) == Some(&b':') {
            return PathBuf::from(drive_path);
        }
    }
    path.to_owned()
}

fn certificate_fingerprint(output: &str) -> Result<String, String> {
    let digest = output
        .lines()
        .find_map(|line| line.strip_prefix("Signer #1 certificate SHA-256 digest:"))
        .map(str::trim)
        .filter(|digest| digest.len() == 64 && digest.bytes().all(|ch| ch.is_ascii_hexdigit()))
        .ok_or("APK imza sertifikasının SHA-256 parmak izi okunamadı")?;
    Ok(digest
        .as_bytes()
        .chunks(2)
        .map(|byte| {
            std::str::from_utf8(byte)
                .expect("hexadecimal data is ASCII")
                .to_ascii_uppercase()
        })
        .collect::<Vec<_>>()
        .join(":"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_origin_and_encodes_entry_filename() {
        let options = Options::new(
            "https://Example.COM/chad/demo/",
            "org.chad.demo".into(),
            true,
        )
        .unwrap();
        assert_eq!(options.origin, "https://example.com");
        assert_eq!(options.directory, "chad/demo");
        assert_eq!(
            options.start_url("Çalışma #1.html"),
            "https://example.com/chad/demo/%C3%87al%C4%B1%C5%9Fma%20%231.html"
        );
    }

    #[test]
    fn rejects_ambiguous_sites_and_invalid_application_ids() {
        for site in [
            "http://example.com",
            "https://example.com/?x=1",
            "https://example.com/a/../b",
            "https://example.com@elsewhere.example/",
            "https://127.0.0.1/",
        ] {
            assert!(
                Options::new(site, "org.chad.demo".into(), true).is_err(),
                "{site}"
            );
        }
        assert!(Options::new("https://example.com/", "org.2chad".into(), true).is_err());
    }

    #[test]
    fn reads_the_apk_signing_certificate_sha256() {
        let digest = "0a".repeat(32);
        let signer_output = format!(
            "Signer #1 certificate DN: CN=Android Debug\r\nSigner #1 certificate SHA-256 digest: {digest}\r\n"
        );
        assert_eq!(
            certificate_fingerprint(&signer_output).unwrap(),
            vec!["0A"; 32].join(":")
        );
        assert!(certificate_fingerprint("Signer #1 certificate SHA-256 digest: f1").is_err());
    }
}
