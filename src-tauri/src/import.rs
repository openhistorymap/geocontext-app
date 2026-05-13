use std::fs;
use std::path::{Path, PathBuf};

use geojson::{Feature, FeatureCollection, GeoJson, Geometry, JsonObject, Value as GValue};
use proj4rs::{transform::transform, Proj};
use regex::Regex;
use serde::Serialize;
use shapefile::Shape;

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("shapefile error: {0}")]
    Shp(String),
    #[error("crs error: {0}")]
    Crs(String),
    #[error("path is not a directory: {0}")]
    NotADirectory(String),
    #[error("path escapes the repository: {0}")]
    PathEscape(String),
    #[error("unsupported shape type")]
    UnsupportedShape,
    #[error("{0}")]
    Other(String),
}

impl From<shapefile::Error> for ImportError {
    fn from(e: shapefile::Error) -> Self {
        ImportError::Shp(e.to_string())
    }
}

#[derive(Debug, Serialize)]
pub struct PrjInfo {
    /// Resolved EPSG code (best-effort: AUTHORITY tag first, then keyword heuristic).
    pub epsg: Option<u32>,
    /// First name token from the WKT (PROJCS / GEOGCS / ...).
    pub name: Option<String>,
    /// Whether a .prj sibling was found at all.
    pub prj_present: bool,
    /// Raw WKT contents, empty string when no .prj is present.
    pub raw_wkt: String,
}

#[derive(Debug, Serialize)]
pub struct ImportedAsset {
    pub rel_path: String,
    pub abs_path: String,
    pub kind: String,
    pub size: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum AssetKind {
    Dataset,
    Image,
    Html,
    Download,
    Other,
}

impl AssetKind {
    pub fn dir(self) -> &'static str {
        match self {
            AssetKind::Dataset => "datasets",
            AssetKind::Image => "images",
            AssetKind::Html => "html",
            AssetKind::Download => "downloads",
            AssetKind::Other => "assets",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            AssetKind::Dataset => "dataset",
            AssetKind::Image => "image",
            AssetKind::Html => "html",
            AssetKind::Download => "download",
            AssetKind::Other => "other",
        }
    }

    pub fn from_extension(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "geojson" | "json" | "shp" => AssetKind::Dataset,
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "tif" | "tiff" | "avif" => {
                AssetKind::Image
            }
            "html" | "htm" => AssetKind::Html,
            "pdf" | "docx" | "doc" | "zip" | "csv" | "txt" | "tsv" | "xlsx" | "rtf" => {
                AssetKind::Download
            }
            _ => AssetKind::Other,
        }
    }

    pub fn from_kind_str(s: &str) -> Self {
        match s {
            "dataset" => AssetKind::Dataset,
            "image" => AssetKind::Image,
            "html" => AssetKind::Html,
            "download" => AssetKind::Download,
            _ => AssetKind::Other,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers

pub fn sanitize_stem(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' => c,
            _ => '_',
        })
        .collect();
    let trimmed = cleaned.trim_matches('.');
    if trimmed.is_empty() {
        "asset".to_string()
    } else {
        trimmed.to_string()
    }
}

fn join_unique(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut candidate = dir.join(format!("{stem}.{ext}"));
    let mut counter: u32 = 1;
    while candidate.exists() {
        candidate = dir.join(format!("{stem}_{counter}.{ext}"));
        counter += 1;
    }
    candidate
}

/// Resolve a child path safely inside `root`. Rejects ".." traversal.
pub fn resolve_inside(root: &Path, rel: &str) -> Result<PathBuf, ImportError> {
    let target = root.join(rel);
    // Canonicalise the root, and the parent of the target (in case target doesn't exist).
    let root_c = root
        .canonicalize()
        .map_err(|e| ImportError::Other(format!("canonicalize repo: {e}")))?;
    let parent = target
        .parent()
        .ok_or_else(|| ImportError::PathEscape(rel.into()))?;
    let parent_c = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());
    if !parent_c.starts_with(&root_c) {
        return Err(ImportError::PathEscape(rel.into()));
    }
    Ok(target)
}

// ---------------------------------------------------------------------------
// .prj inspection

pub fn read_prj_for(shp_path: &Path) -> Result<PrjInfo, ImportError> {
    let prj_path = shp_path.with_extension("prj");
    if !prj_path.exists() {
        return Ok(PrjInfo {
            epsg: None,
            name: None,
            prj_present: false,
            raw_wkt: String::new(),
        });
    }
    let raw = fs::read_to_string(&prj_path)?;
    let epsg = epsg_from_wkt(&raw);
    let name = wkt_top_name(&raw);
    Ok(PrjInfo {
        epsg,
        name,
        prj_present: true,
        raw_wkt: raw,
    })
}

fn epsg_from_wkt(wkt: &str) -> Option<u32> {
    // Last AUTHORITY["EPSG","NNNN"] entry wins — it's the most specific identifier.
    if let Ok(re) = Regex::new(r#"AUTHORITY\s*\[\s*"EPSG"\s*,\s*"?(\d+)"?\s*\]"#) {
        if let Some(code) = re
            .captures_iter(wkt)
            .filter_map(|c| c.get(1).and_then(|m| m.as_str().parse::<u32>().ok()))
            .last()
        {
            return Some(code);
        }
    }
    // Fallback: simple keyword heuristic for files that omit AUTHORITY.
    let lower = wkt.to_ascii_lowercase();
    if lower.contains("web_mercator") || lower.contains("pseudo-mercator") {
        return Some(3857);
    }
    if lower.contains("wgs_1984") || lower.contains("wgs 84") || lower.contains("wgs84") {
        return Some(4326);
    }
    None
}

fn wkt_top_name(wkt: &str) -> Option<String> {
    let re = Regex::new(r#"^\s*[A-Z]+CS\s*\[\s*"([^"]+)""#).ok()?;
    re.captures(wkt)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

// ---------------------------------------------------------------------------
// GeoJSON import — copy + JSON-validate

pub fn import_geojson_into(
    repo: &Path,
    src: &Path,
    name_override: Option<&str>,
) -> Result<ImportedAsset, ImportError> {
    let raw = fs::read_to_string(src)?;
    let _: serde_json::Value = serde_json::from_str(&raw)?;

    let datasets = repo.join("datasets");
    fs::create_dir_all(&datasets)?;

    let stem = name_override
        .map(str::to_string)
        .or_else(|| src.file_stem().and_then(|s| s.to_str()).map(str::to_string))
        .unwrap_or_else(|| "dataset".to_string());
    let safe = sanitize_stem(&stem);
    let target = join_unique(&datasets, &safe, "geojson");

    fs::write(&target, raw.as_bytes())?;
    let size = fs::metadata(&target)?.len();
    let rel = target
        .strip_prefix(repo)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| target.display().to_string());

    Ok(ImportedAsset {
        rel_path: rel,
        abs_path: target.display().to_string(),
        kind: AssetKind::Dataset.as_str().into(),
        size,
    })
}

// ---------------------------------------------------------------------------
// Arbitrary asset import (images / html / downloads / other)

pub fn import_asset_into(
    repo: &Path,
    src: &Path,
    kind: AssetKind,
    name_override: Option<&str>,
) -> Result<ImportedAsset, ImportError> {
    let dir = repo.join(kind.dir());
    fs::create_dir_all(&dir)?;

    let stem = name_override
        .map(str::to_string)
        .or_else(|| src.file_stem().and_then(|s| s.to_str()).map(str::to_string))
        .unwrap_or_else(|| "asset".to_string());
    let safe = sanitize_stem(&stem);
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| "bin".to_string());
    let target = join_unique(&dir, &safe, &ext);
    fs::copy(src, &target)?;

    let size = fs::metadata(&target)?.len();
    let rel = target
        .strip_prefix(repo)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| target.display().to_string());

    Ok(ImportedAsset {
        rel_path: rel,
        abs_path: target.display().to_string(),
        kind: kind.as_str().into(),
        size,
    })
}

// ---------------------------------------------------------------------------
// SHP → GeoJSON (reprojected to EPSG:4326)

pub fn import_shp_into(
    repo: &Path,
    src: &Path,
    source_epsg: u32,
    name_override: Option<&str>,
) -> Result<ImportedAsset, ImportError> {
    let src_proj = Proj::from_user_string(&format!("EPSG:{source_epsg}"))
        .map_err(|e| ImportError::Crs(format!("EPSG:{source_epsg}: {e}")))?;
    let dst_proj = Proj::from_user_string("EPSG:4326")
        .map_err(|e| ImportError::Crs(format!("EPSG:4326: {e}")))?;

    let mut reader = shapefile::Reader::from_path(src)?;
    let mut features: Vec<Feature> = Vec::new();
    for record in reader.iter_shapes_and_records() {
        let (shape, dbf) = record?;
        let geometry = shape_to_geometry(&shape, &src_proj, &dst_proj)?;
        if geometry.is_none() {
            continue;
        }
        let mut properties = JsonObject::new();
        for (field, value) in dbf {
            properties.insert(field, dbf_value_to_json(value));
        }
        features.push(Feature {
            bbox: None,
            geometry,
            id: None,
            properties: Some(properties),
            foreign_members: None,
        });
    }

    let fc = FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };
    let serialized = serde_json::to_string_pretty(&GeoJson::FeatureCollection(fc))?;

    let datasets = repo.join("datasets");
    fs::create_dir_all(&datasets)?;
    let stem = name_override
        .map(str::to_string)
        .or_else(|| src.file_stem().and_then(|s| s.to_str()).map(str::to_string))
        .unwrap_or_else(|| "dataset".to_string());
    let safe = sanitize_stem(&stem);
    let target = join_unique(&datasets, &safe, "geojson");
    fs::write(&target, serialized.as_bytes())?;

    let size = fs::metadata(&target)?.len();
    let rel = target
        .strip_prefix(repo)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| target.display().to_string());
    Ok(ImportedAsset {
        rel_path: rel,
        abs_path: target.display().to_string(),
        kind: AssetKind::Dataset.as_str().into(),
        size,
    })
}

fn shape_to_geometry(
    shape: &Shape,
    src: &Proj,
    dst: &Proj,
) -> Result<Option<Geometry>, ImportError> {
    let value = match shape {
        Shape::NullShape => return Ok(None),

        Shape::Point(p) => GValue::Point(reproject_xy(src, dst, p.x, p.y)?),
        Shape::PointZ(p) => GValue::Point(reproject_xy(src, dst, p.x, p.y)?),
        Shape::PointM(p) => GValue::Point(reproject_xy(src, dst, p.x, p.y)?),

        Shape::Multipoint(mp) => {
            let pts = mp
                .points()
                .iter()
                .map(|p| reproject_xy(src, dst, p.x, p.y))
                .collect::<Result<Vec<_>, _>>()?;
            GValue::MultiPoint(pts)
        }
        Shape::MultipointZ(mp) => {
            let pts = mp
                .points()
                .iter()
                .map(|p| reproject_xy(src, dst, p.x, p.y))
                .collect::<Result<Vec<_>, _>>()?;
            GValue::MultiPoint(pts)
        }
        Shape::MultipointM(mp) => {
            let pts = mp
                .points()
                .iter()
                .map(|p| reproject_xy(src, dst, p.x, p.y))
                .collect::<Result<Vec<_>, _>>()?;
            GValue::MultiPoint(pts)
        }

        Shape::Polyline(pl) => {
            let lines = pl
                .parts()
                .iter()
                .map(|part| {
                    part.iter()
                        .map(|p| reproject_xy(src, dst, p.x, p.y))
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()?;
            if lines.len() == 1 {
                GValue::LineString(lines.into_iter().next().unwrap())
            } else {
                GValue::MultiLineString(lines)
            }
        }
        Shape::PolylineZ(pl) => {
            let lines = pl
                .parts()
                .iter()
                .map(|part| {
                    part.iter()
                        .map(|p| reproject_xy(src, dst, p.x, p.y))
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()?;
            if lines.len() == 1 {
                GValue::LineString(lines.into_iter().next().unwrap())
            } else {
                GValue::MultiLineString(lines)
            }
        }
        Shape::PolylineM(pl) => {
            let lines = pl
                .parts()
                .iter()
                .map(|part| {
                    part.iter()
                        .map(|p| reproject_xy(src, dst, p.x, p.y))
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()?;
            if lines.len() == 1 {
                GValue::LineString(lines.into_iter().next().unwrap())
            } else {
                GValue::MultiLineString(lines)
            }
        }

        Shape::Polygon(pg) => GValue::MultiPolygon(polygon_rings(pg.rings(), src, dst)?),
        Shape::PolygonZ(pg) => GValue::MultiPolygon(polygon_rings(pg.rings(), src, dst)?),
        Shape::PolygonM(pg) => GValue::MultiPolygon(polygon_rings(pg.rings(), src, dst)?),

        // Multipatch is approximated as a polygon collection of its outer rings;
        // most authors don't ship them in GeoContext-shaped datasets anyway.
        _ => return Err(ImportError::UnsupportedShape),
    };
    Ok(Some(Geometry::new(value)))
}

/// Build GeoJSON-shape MultiPolygon coords out of shapefile rings.
/// An "outer" ring starts a polygon; subsequent "inner" rings are holes
/// attached to the most recent polygon.
fn polygon_rings<P>(
    rings: &[shapefile::PolygonRing<P>],
    src: &Proj,
    dst: &Proj,
) -> Result<Vec<geojson::PolygonType>, ImportError>
where
    P: HasXY,
{
    use shapefile::PolygonRing;

    let mut out: Vec<geojson::PolygonType> = Vec::new();
    for ring in rings {
        let coords = match ring {
            PolygonRing::Outer(points) | PolygonRing::Inner(points) => points
                .iter()
                .map(|p| reproject_xy(src, dst, p.x(), p.y()))
                .collect::<Result<Vec<_>, _>>()?,
        };
        match ring {
            PolygonRing::Outer(_) => out.push(vec![coords]),
            PolygonRing::Inner(_) => {
                if let Some(last) = out.last_mut() {
                    last.push(coords);
                } else {
                    out.push(vec![coords]);
                }
            }
        }
    }
    Ok(out)
}

/// Minimal accessor trait so polygon_rings can iterate over Point/PointZ/PointM uniformly.
trait HasXY {
    fn x(&self) -> f64;
    fn y(&self) -> f64;
}
impl HasXY for shapefile::Point {
    fn x(&self) -> f64 {
        self.x
    }
    fn y(&self) -> f64 {
        self.y
    }
}
impl HasXY for shapefile::PointZ {
    fn x(&self) -> f64 {
        self.x
    }
    fn y(&self) -> f64 {
        self.y
    }
}
impl HasXY for shapefile::PointM {
    fn x(&self) -> f64 {
        self.x
    }
    fn y(&self) -> f64 {
        self.y
    }
}

fn reproject_xy(src: &Proj, dst: &Proj, x: f64, y: f64) -> Result<Vec<f64>, ImportError> {
    // proj4rs uses radians for geographic CRSes; SHPs store degrees for those.
    let mut pt: (f64, f64, f64) = (x, y, 0.0);
    if src.is_latlong() {
        pt.0 = pt.0.to_radians();
        pt.1 = pt.1.to_radians();
    }
    transform(src, dst, &mut pt).map_err(|e| ImportError::Crs(e.to_string()))?;
    if dst.is_latlong() {
        pt.0 = pt.0.to_degrees();
        pt.1 = pt.1.to_degrees();
    }
    Ok(vec![pt.0, pt.1])
}

fn dbf_value_to_json(value: shapefile::dbase::FieldValue) -> serde_json::Value {
    use serde_json::Value as J;
    use shapefile::dbase::FieldValue::*;
    match value {
        Character(s) => J::String(s.unwrap_or_default()),
        Numeric(n) => n
            .and_then(|x| serde_json::Number::from_f64(x).map(J::Number))
            .unwrap_or(J::Null),
        Logical(b) => b.map(J::Bool).unwrap_or(J::Null),
        Date(d) => d.map(|x| J::String(x.to_string())).unwrap_or(J::Null),
        Float(f) => f
            .and_then(|x| serde_json::Number::from_f64(x as f64).map(J::Number))
            .unwrap_or(J::Null),
        Integer(i) => J::Number(i.into()),
        Currency(c) => serde_json::Number::from_f64(c)
            .map(J::Number)
            .unwrap_or(J::Null),
        DateTime(dt) => J::String(format!("{dt:?}")),
        Double(d) => serde_json::Number::from_f64(d)
            .map(J::Number)
            .unwrap_or(J::Null),
        Memo(s) => J::String(s),
    }
}

// ---------------------------------------------------------------------------
// Asset listing — recursive walk with kind + size + modified

#[derive(Debug, Serialize)]
pub struct RepoAsset {
    pub rel_path: String,
    pub abs_path: String,
    pub kind: String,
    pub size: u64,
    pub modified_secs: Option<u64>,
}

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".svelte-kit",
    "build",
    "dist",
    "target",
    ".idea",
    ".vscode",
];

pub fn list_assets_recursive(root: &Path) -> std::io::Result<Vec<RepoAsset>> {
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(out)
}

fn walk(root: &Path, cur: &Path, out: &mut Vec<RepoAsset>) -> std::io::Result<()> {
    let name = cur.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if SKIP_DIRS.contains(&name) {
        return Ok(());
    }
    for entry in fs::read_dir(cur)? {
        let entry = entry?;
        let p = entry.path();
        let meta = entry.metadata()?;
        if meta.is_dir() {
            walk(root, &p, out)?;
            continue;
        }
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let kind = AssetKind::from_extension(&ext);
        let rel = p
            .strip_prefix(root)
            .map(|r| r.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| p.display().to_string());
        // Skip the geocontext / gcx files themselves — they aren't "assets".
        if rel == "geocontext.json" || rel == "gcx.json" {
            continue;
        }
        let modified_secs = meta
            .modified()
            .ok()
            .and_then(|mt| mt.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        out.push(RepoAsset {
            rel_path: rel,
            abs_path: p.display().to_string(),
            kind: kind.as_str().into(),
            size: meta.len(),
            modified_secs,
        });
    }
    Ok(())
}
