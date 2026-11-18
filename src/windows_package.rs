use super::{context, write_runtime};
use std::{ffi::c_void, io, os::windows::ffi::OsStrExt, path::Path};

pub const DEFAULT_ICON: &[u8] = include_bytes!("../assets/chad.ico");
pub const ICON_EXTENSION: &str = "ico";
pub const ICON_MAGIC: &[u8] = &[0, 0, 1, 0];

pub fn directory_name(name: &str) -> String {
    name.to_owned()
}
pub fn validate_output(_: &Path) -> Result<(), String> {
    Ok(())
}
pub fn files_directory(output: &Path) -> std::path::PathBuf {
    output.to_owned()
}

pub fn finish(output: &Path, name: &str, icon: Option<&[u8]>) -> Result<(), String> {
    let executable = output.join(format!("{name}.exe"));
    write_runtime(&executable)?;
    if let Some(icon) = icon {
        embed_icon(&executable, icon).map_err(|error| context(&executable, error))?;
    }
    Ok(())
}

fn embed_icon(executable: &Path, icon: &[u8]) -> io::Result<()> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "geçersiz .ico verisi");
    let count = icon
        .get(4..6)
        .map(|data| u16::from_le_bytes([data[0], data[1]]))
        .ok_or_else(invalid)?;
    if count == 0 {
        return Err(invalid());
    }
    let table_end = 6 + usize::from(count) * 16;
    let entries = icon.get(6..table_end).ok_or_else(invalid)?;
    let mut images = Vec::with_capacity(usize::from(count));
    let mut group = icon[..6].to_vec();

    for (index, entry) in entries.chunks_exact(16).enumerate() {
        let size = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
        let start = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
        let end = start.checked_add(size).ok_or_else(invalid)?;
        if size == 0 || start < table_end {
            return Err(invalid());
        }
        let image = icon.get(start..end).ok_or_else(invalid)?;
        let id = (index + 1) as u16;
        images.push((id, image));
        group.extend_from_slice(&entry[..12]);
        group.extend_from_slice(&id.to_le_bytes());
    }

    let path: Vec<u16> = executable.as_os_str().encode_wide().chain([0]).collect();
    // Windows splits the ICO directory and its images into separate PE resources.
    let update = unsafe { BeginUpdateResourceW(path.as_ptr(), 0) };
    if update.is_null() {
        return Err(io::Error::last_os_error());
    }
    let result = (|| {
        for (id, image) in images {
            update_resource(update, 3, id, image)?;
        }
        update_resource(update, 14, 1, &group)
    })();
    // EndUpdateResourceW also releases the handle when pending changes are discarded.
    let finished = unsafe { EndUpdateResourceW(update, i32::from(result.is_err())) };
    result?;
    if finished == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn update_resource(update: *mut c_void, kind: u16, id: u16, bytes: &[u8]) -> io::Result<()> {
    // Win32 represents a numeric resource identifier as a pointer-sized integer.
    let ok = unsafe {
        UpdateResourceW(
            update,
            usize::from(kind) as *const u16,
            usize::from(id) as *const u16,
            0,
            bytes.as_ptr().cast(),
            bytes.len() as u32,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn BeginUpdateResourceW(file: *const u16, delete_existing: i32) -> *mut c_void;
    fn UpdateResourceW(
        update: *mut c_void,
        kind: *const u16,
        name: *const u16,
        language: u16,
        data: *const c_void,
        length: u32,
    ) -> i32;
    fn EndUpdateResourceW(update: *mut c_void, discard: i32) -> i32;
}
