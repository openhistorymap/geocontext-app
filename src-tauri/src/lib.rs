use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("path is not a directory: {0}")]
    NotADirectory(String),
    #[error("no geocontext.json or gcx.json found in {0}")]
    NotFound(String),
    #[error("invalid filename {0:?}; allowed: geocontext.json, gcx.json")]
    InvalidFilename(String),
    #[error("file already contains a different geocontext file: {0}")]
    AlreadyExists(String),
}

impl serde::Serialize for AppErrorRepr {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.0)
    }
}

// Tauri commands must return a serializable error. We wrap into a string repr.
pub struct AppErrorRepr(String);

impl From<AppError> for AppErrorRepr {
    fn from(e: AppError) -> Self {
        AppErrorRepr(e.to_string())
    }
}

type CmdResult<T> = Result<T, AppErrorRepr>;

#[derive(Debug, Serialize)]
pub struct LoadedGeoContext {
    pub folder: String,
    pub filename: String,
    pub content: String,
}

const ALLOWED: &[&str] = &["geocontext.json", "gcx.json"];

fn ensure_dir(folder: &str) -> Result<PathBuf, AppError> {
    let p = PathBuf::from(folder);
    if !p.is_dir() {
        return Err(AppError::NotADirectory(folder.to_string()));
    }
    Ok(p)
}

#[tauri::command]
fn load_geocontext(folder: String) -> CmdResult<LoadedGeoContext> {
    let dir = ensure_dir(&folder).map_err(AppErrorRepr::from)?;
    for &name in ALLOWED {
        let p = dir.join(name);
        if p.is_file() {
            let content =
                fs::read_to_string(&p).map_err(|e| AppErrorRepr::from(AppError::Io(e)))?;
            // Validate JSON now so the frontend can rely on it
            let _: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| AppErrorRepr::from(AppError::Json(e)))?;
            return Ok(LoadedGeoContext {
                folder,
                filename: name.to_string(),
                content,
            });
        }
    }
    Err(AppErrorRepr::from(AppError::NotFound(folder)))
}

#[tauri::command]
fn save_geocontext(folder: String, filename: String, content: String) -> CmdResult<()> {
    if !ALLOWED.contains(&filename.as_str()) {
        return Err(AppErrorRepr::from(AppError::InvalidFilename(filename)));
    }
    let dir = ensure_dir(&folder).map_err(AppErrorRepr::from)?;
    let _: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| AppErrorRepr::from(AppError::Json(e)))?;
    let target = dir.join(&filename);
    atomic_write(&target, content.as_bytes()).map_err(|e| AppErrorRepr::from(AppError::Io(e)))?;
    Ok(())
}

#[tauri::command]
fn create_geocontext_repo(folder: String, filename: String, content: String) -> CmdResult<()> {
    if !ALLOWED.contains(&filename.as_str()) {
        return Err(AppErrorRepr::from(AppError::InvalidFilename(filename)));
    }
    let dir = PathBuf::from(&folder);
    fs::create_dir_all(&dir).map_err(|e| AppErrorRepr::from(AppError::Io(e)))?;
    for &name in ALLOWED {
        if dir.join(name).exists() {
            return Err(AppErrorRepr::from(AppError::AlreadyExists(
                dir.join(name).display().to_string(),
            )));
        }
    }
    let _: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| AppErrorRepr::from(AppError::Json(e)))?;
    atomic_write(&dir.join(&filename), content.as_bytes())
        .map_err(|e| AppErrorRepr::from(AppError::Io(e)))?;
    // Seed an empty datasets/ so paths in §9 have a home
    let datasets = dir.join("datasets");
    if !datasets.exists() {
        fs::create_dir_all(&datasets).map_err(|e| AppErrorRepr::from(AppError::Io(e)))?;
    }
    Ok(())
}

#[tauri::command]
fn list_assets(folder: String) -> CmdResult<Vec<String>> {
    let dir = ensure_dir(&folder).map_err(AppErrorRepr::from)?;
    let mut out = Vec::new();
    walk(&dir, &dir, &mut out).map_err(|e| AppErrorRepr::from(AppError::Io(e)))?;
    out.sort();
    Ok(out)
}

fn walk(root: &Path, cur: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
    // Cap traversal so we don't list .git etc.
    let name = cur.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if matches!(
        name,
        ".git" | "node_modules" | ".svelte-kit" | "build" | "dist" | "target"
    ) {
        return Ok(());
    }
    for entry in fs::read_dir(cur)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            walk(root, &p, out)?;
        } else if let Ok(rel) = p.strip_prefix(root) {
            out.push(rel.to_string_lossy().into_owned());
        }
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|s| s.to_str()).unwrap_or("")
    ));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_geocontext,
            save_geocontext,
            create_geocontext_repo,
            list_assets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
