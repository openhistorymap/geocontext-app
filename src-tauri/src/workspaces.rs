use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// A known GeoContext repository the user has worked with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Absolute path to the repository root.
    pub path: String,
    /// Optional title carried from geocontext.json's `title` field.
    pub title: Option<String>,
    /// "geocontext.json" or "gcx.json".
    pub filename: Option<String>,
    /// Unix seconds.
    pub last_opened_unix: u64,
}

/// Wire-shape returned to the frontend — adds a derived `reachable` flag.
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceEntry {
    #[serde(flatten)]
    pub ws: Workspace,
    pub reachable: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceList {
    pub workspaces: Vec<Workspace>,
}

/// Maximum number of entries we keep on disk.
const MAX: usize = 32;

pub fn workspaces_path(config_dir: &Path) -> PathBuf {
    config_dir.join("workspaces.json")
}

pub fn load(config_dir: &Path) -> std::io::Result<WorkspaceList> {
    let path = workspaces_path(config_dir);
    if !path.exists() {
        return Ok(WorkspaceList::default());
    }
    let raw = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

pub fn save(config_dir: &Path, list: &WorkspaceList) -> std::io::Result<()> {
    fs::create_dir_all(config_dir)?;
    let path = workspaces_path(config_dir);
    let tmp = path.with_extension("json.tmp");
    let serialized = serde_json::to_string_pretty(list).unwrap_or_else(|_| "{}".to_string());
    fs::write(&tmp, serialized.as_bytes())?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Upsert + bump last_opened_unix. Returns the resulting list.
pub fn touch(
    config_dir: &Path,
    path: String,
    title: Option<String>,
    filename: Option<String>,
) -> std::io::Result<WorkspaceList> {
    let mut list = load(config_dir)?;
    list.workspaces.retain(|w| w.path != path);
    list.workspaces.insert(
        0,
        Workspace {
            path,
            title,
            filename,
            last_opened_unix: now_secs(),
        },
    );
    list.workspaces.truncate(MAX);
    save(config_dir, &list)?;
    Ok(list)
}

pub fn remove(config_dir: &Path, path: &str) -> std::io::Result<WorkspaceList> {
    let mut list = load(config_dir)?;
    list.workspaces.retain(|w| w.path != path);
    save(config_dir, &list)?;
    Ok(list)
}

/// Drop any entry whose path is no longer a directory.
pub fn forget_unreachable(config_dir: &Path) -> std::io::Result<WorkspaceList> {
    let mut list = load(config_dir)?;
    list.workspaces.retain(|w| Path::new(&w.path).is_dir());
    save(config_dir, &list)?;
    Ok(list)
}

pub fn annotate(list: WorkspaceList) -> Vec<WorkspaceEntry> {
    list.workspaces
        .into_iter()
        .map(|ws| {
            let reachable = Path::new(&ws.path).is_dir();
            WorkspaceEntry { ws, reachable }
        })
        .collect()
}
