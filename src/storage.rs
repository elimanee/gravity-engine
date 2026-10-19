//! The player's files (settings, levels, challenges, scenes, the endless
//! run's ghost): plain files on the desktop; in the web version the same
//! paths are keys in the browser's local storage (see `web/gravity_web.js`),
//! under a made-up config folder, so the rest of the program is the same.

use std::path::{Path, PathBuf};

/// Read a whole file.
pub fn read(path: &Path) -> Result<Vec<u8>, String> {
    platform::read(path)
}

pub fn read_to_string(path: &Path) -> Result<String, String> {
    String::from_utf8(read(path)?).map_err(|e| e.to_string())
}

/// Write a whole file, making its folder first.
pub fn write(path: &Path, data: impl AsRef<[u8]>) -> Result<(), String> {
    platform::write(path, data.as_ref())
}

pub fn exists(path: &Path) -> bool {
    platform::exists(path)
}

/// The files directly in `dir`.
pub fn list(dir: &Path) -> Vec<PathBuf> {
    platform::list(dir)
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use std::path::{Path, PathBuf};

    pub fn read(path: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|e| e.to_string())
    }

    pub fn write(path: &Path, data: &[u8]) -> Result<(), String> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, data).map_err(|e| e.to_string())
    }

    pub fn exists(path: &Path) -> bool {
        path.exists()
    }

    pub fn list(dir: &Path) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
        entries.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect()
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use std::path::{Path, PathBuf};

    #[link(wasm_import_module = "env")]
    extern "C" {
        /// What = 0: the value under the key; 1: the keys starting with it,
        /// one per line. Returns its length in bytes (-1: none) and keeps it
        /// for `ge_storage_fetch`.
        fn ge_storage_query(what: u32, key: *const u8, key_len: u32) -> i32;
        fn ge_storage_fetch(dest: *mut u8);
        /// 1 when stored (0: the browser's storage is full or off).
        fn ge_storage_set(key: *const u8, key_len: u32, value: *const u8, value_len: u32) -> u32;
    }

    fn key(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    fn query(what: u32, k: &str) -> Option<Vec<u8>> {
        // SAFETY: the key is read during the call; the result is copied
        // into a buffer of exactly the length the browser reported.
        unsafe {
            let n = ge_storage_query(what, k.as_ptr(), k.len() as u32);
            if n < 0 {
                return None;
            }
            let mut buf = vec![0u8; n as usize];
            ge_storage_fetch(buf.as_mut_ptr());
            Some(buf)
        }
    }

    pub fn read(path: &Path) -> Result<Vec<u8>, String> {
        query(0, &key(path)).ok_or_else(|| "not found".to_string())
    }

    pub fn write(path: &Path, data: &[u8]) -> Result<(), String> {
        let k = key(path);
        // SAFETY: both buffers are read during the call.
        let ok = unsafe { ge_storage_set(k.as_ptr(), k.len() as u32, data.as_ptr(), data.len() as u32) };
        if ok == 1 {
            Ok(())
        } else {
            Err("the browser's storage is full or turned off".into())
        }
    }

    pub fn exists(path: &Path) -> bool {
        query(0, &key(path)).is_some()
    }

    /// Keys under `dir/` with no further `/`.
    pub fn list(dir: &Path) -> Vec<PathBuf> {
        let prefix = format!("{}/", key(dir).trim_end_matches('/'));
        let Some(keys) = query(1, &prefix) else { return vec![] };
        String::from_utf8_lossy(&keys)
            .lines()
            .filter(|k| k.len() > prefix.len() && !k[prefix.len()..].contains('/'))
            .map(PathBuf::from)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_round_trip_and_list() {
        let dir = std::env::temp_dir().join(format!("ge-storage-{}", std::process::id()));
        let a = dir.join("levels").join("a.gscene");
        write(&a, "{\"x\":1}").unwrap();
        write(dir.join("levels").join("b.gscene").as_path(), b"{}").unwrap();
        assert!(exists(&a));
        assert_eq!(read_to_string(&a).unwrap(), "{\"x\":1}");
        let mut names: Vec<String> =
            list(&dir.join("levels")).iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        names.sort();
        assert_eq!(names, ["a.gscene", "b.gscene"]);
        assert!(list(&dir).is_empty(), "only files directly in the folder");
        std::fs::remove_dir_all(&dir).ok();
        assert!(!exists(&a));
    }
}
