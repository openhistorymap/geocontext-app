use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

mod import;
mod workspaces;

use import::{AssetKind, ImportError, ImportedAsset, PrjInfo, RepoAsset};
use tauri::Manager;
use workspaces::WorkspaceEntry;

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
    #[error("import: {0}")]
    Import(#[from] ImportError),
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

impl From<ImportError> for AppErrorRepr {
    fn from(e: ImportError) -> Self {
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
    let datasets = dir.join("datasets");
    if !datasets.exists() {
        fs::create_dir_all(&datasets).map_err(|e| AppErrorRepr::from(AppError::Io(e)))?;
    }
    Ok(())
}

#[tauri::command]
fn list_assets(folder: String) -> CmdResult<Vec<String>> {
    let dir = ensure_dir(&folder).map_err(AppErrorRepr::from)?;
    let mut out: Vec<String> = import::list_assets_recursive(&dir)
        .map_err(|e| AppErrorRepr::from(AppError::Io(e)))?
        .into_iter()
        .map(|a| a.rel_path)
        .collect();
    out.sort();
    Ok(out)
}

#[tauri::command]
fn list_repo_assets(folder: String) -> CmdResult<Vec<RepoAsset>> {
    let dir = ensure_dir(&folder).map_err(AppErrorRepr::from)?;
    import::list_assets_recursive(&dir).map_err(|e| AppErrorRepr::from(AppError::Io(e)))
}

#[tauri::command]
fn detect_shp_crs(shp_path: String) -> CmdResult<PrjInfo> {
    let p = PathBuf::from(shp_path);
    import::read_prj_for(&p).map_err(AppErrorRepr::from)
}

#[tauri::command]
fn import_geojson_local(
    folder: String,
    src_path: String,
    name_override: Option<String>,
) -> CmdResult<ImportedAsset> {
    let dir = ensure_dir(&folder).map_err(AppErrorRepr::from)?;
    let src = PathBuf::from(src_path);
    Ok(import::import_geojson_into(
        &dir,
        &src,
        name_override.as_deref(),
    )?)
}

#[tauri::command]
fn import_shp_local(
    folder: String,
    src_path: String,
    source_epsg: u32,
    name_override: Option<String>,
) -> CmdResult<ImportedAsset> {
    let dir = ensure_dir(&folder).map_err(AppErrorRepr::from)?;
    let src = PathBuf::from(src_path);
    Ok(import::import_shp_into(
        &dir,
        &src,
        source_epsg,
        name_override.as_deref(),
    )?)
}

#[tauri::command]
fn import_asset_local(
    folder: String,
    src_path: String,
    kind: String,
    name_override: Option<String>,
) -> CmdResult<ImportedAsset> {
    let dir = ensure_dir(&folder).map_err(AppErrorRepr::from)?;
    let src = PathBuf::from(src_path);
    let k = AssetKind::from_kind_str(&kind);
    Ok(import::import_asset_into(
        &dir,
        &src,
        k,
        name_override.as_deref(),
    )?)
}

#[tauri::command]
fn delete_repo_asset(folder: String, rel_path: String) -> CmdResult<()> {
    if rel_path.is_empty() || rel_path.contains("..") {
        return Err(AppErrorRepr::from(AppError::Import(
            ImportError::PathEscape(rel_path),
        )));
    }
    // Refuse to delete the geocontext file itself.
    if rel_path == "geocontext.json" || rel_path == "gcx.json" {
        return Err(AppErrorRepr::from(AppError::Import(ImportError::Other(
            "refusing to delete the geocontext file via the asset browser".into(),
        ))));
    }
    let dir = ensure_dir(&folder).map_err(AppErrorRepr::from)?;
    let target = import::resolve_inside(&dir, &rel_path).map_err(AppErrorRepr::from)?;
    if !target.exists() {
        return Err(AppErrorRepr::from(AppError::NotFound(rel_path)));
    }
    fs::remove_file(&target).map_err(|e| AppErrorRepr::from(AppError::Io(e)))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Workspaces (recent / pinned repositories)

fn config_dir(app: &tauri::AppHandle) -> Result<PathBuf, AppErrorRepr> {
    app.path()
        .app_config_dir()
        .map_err(|e| AppErrorRepr(format!("could not resolve app config dir: {e}")))
}

#[tauri::command]
fn list_workspaces(app: tauri::AppHandle) -> CmdResult<Vec<WorkspaceEntry>> {
    let dir = config_dir(&app)?;
    let list = workspaces::load(&dir).map_err(|e| AppErrorRepr::from(AppError::Io(e)))?;
    Ok(workspaces::annotate(list))
}

#[tauri::command]
fn touch_workspace(
    app: tauri::AppHandle,
    path: String,
    title: Option<String>,
    filename: Option<String>,
) -> CmdResult<Vec<WorkspaceEntry>> {
    let dir = config_dir(&app)?;
    let list = workspaces::touch(&dir, path, title, filename)
        .map_err(|e| AppErrorRepr::from(AppError::Io(e)))?;
    Ok(workspaces::annotate(list))
}

#[tauri::command]
fn remove_workspace(app: tauri::AppHandle, path: String) -> CmdResult<Vec<WorkspaceEntry>> {
    let dir = config_dir(&app)?;
    let list = workspaces::remove(&dir, &path).map_err(|e| AppErrorRepr::from(AppError::Io(e)))?;
    Ok(workspaces::annotate(list))
}

#[tauri::command]
fn forget_unreachable_workspaces(app: tauri::AppHandle) -> CmdResult<Vec<WorkspaceEntry>> {
    let dir = config_dir(&app)?;
    let list =
        workspaces::forget_unreachable(&dir).map_err(|e| AppErrorRepr::from(AppError::Io(e)))?;
    Ok(workspaces::annotate(list))
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
            list_repo_assets,
            detect_shp_crs,
            import_geojson_local,
            import_shp_local,
            import_asset_local,
            delete_repo_asset,
            list_workspaces,
            touch_workspace,
            remove_workspace,
            forget_unreachable_workspaces,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
