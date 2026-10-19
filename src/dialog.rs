//! File dialogs: the system's on the desktop. The web version has no files
//! to pick, so there they come back empty.

#[cfg(not(target_arch = "wasm32"))]
pub use rfd::FileDialog;

#[cfg(target_arch = "wasm32")]
pub struct FileDialog;

#[cfg(target_arch = "wasm32")]
impl FileDialog {
    pub fn new() -> Self {
        FileDialog
    }

    pub fn add_filter(self, _name: impl Into<String>, _extensions: &[impl ToString]) -> Self {
        self
    }

    pub fn set_directory<P: AsRef<std::path::Path>>(self, _path: P) -> Self {
        self
    }

    pub fn set_file_name(self, _name: impl Into<String>) -> Self {
        self
    }

    pub fn pick_file(self) -> Option<std::path::PathBuf> {
        None
    }

    pub fn pick_files(self) -> Option<Vec<std::path::PathBuf>> {
        None
    }

    pub fn save_file(self) -> Option<std::path::PathBuf> {
        None
    }
}
