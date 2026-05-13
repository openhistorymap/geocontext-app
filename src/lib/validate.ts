// Lightweight GeoContext validator. Returns flat issue list keyed by
// dotted path so the UI can surface inline errors. Conservative: only
// flags violations the FORMAT.md spec describes as required.

import type { GeoContext, Datasource, Layer, CsvColumn } from './types';

export type Severity = 'error' | 'warn';

export interface Issue {
  path: string;
  severity: Severity;
  message: string;
}

export function validate(gc: GeoContext): Issue[] {
  const issues: Issue[] = [];

  // -- top-level ---------------------------------------------------------
  if (!gc.title || typeof gc.title !== 'string')
    issues.push({ path: 'title', severity: 'error', message: 'title is required' });

  if (gc.type !== '2d')
    issues.push({ path: 'type', severity: 'warn', message: 'type should be "2d" (reserved)' });

  if (!Array.isArray(gc.center) || gc.center.length !== 2)
    issues.push({ path: 'center', severity: 'error', message: 'center must be [lat, lon]' });
  else {
    const [lat, lon] = gc.center;
    if (typeof lat !== 'number' || lat < -90 || lat > 90)
      issues.push({ path: 'center.0', severity: 'error', message: 'lat must be a number in [-90, 90]' });
    if (typeof lon !== 'number' || lon < -180 || lon > 180)
      issues.push({ path: 'center.1', severity: 'error', message: 'lon must be a number in [-180, 180]' });
  }

  for (const k of ['minzoom', 'startzoom', 'maxzoom'] as const) {
    const v = gc[k];
    if (typeof v !== 'number' || v < 0 || v > 24)
      issues.push({ path: k, severity: 'error', message: `${k} must be a number 0..24` });
  }
  if (
    typeof gc.minzoom === 'number' &&
    typeof gc.maxzoom === 'number' &&
    gc.minzoom > gc.maxzoom
  )
    issues.push({ path: 'maxzoom', severity: 'error', message: 'maxzoom must be >= minzoom' });

  if (
    typeof gc.startzoom === 'number' &&
    typeof gc.minzoom === 'number' &&
    typeof gc.maxzoom === 'number' &&
    (gc.startzoom < gc.minzoom || gc.startzoom > gc.maxzoom)
  )
    issues.push({ path: 'startzoom', severity: 'warn', message: 'startzoom should be in [minzoom, maxzoom]' });

  // -- datasources -------------------------------------------------------
  if (!Array.isArray(gc.datasources))
    issues.push({ path: 'datasources', severity: 'error', message: 'datasources must be an array' });
  else {
    const seen = new Set<string>();
    gc.datasources.forEach((ds, i) => validateDatasource(ds, `datasources.${i}`, issues, seen));
  }

  // -- layers ------------------------------------------------------------
  if (!Array.isArray(gc.layers))
    issues.push({ path: 'layers', severity: 'error', message: 'layers must be an array' });
  else {
    const seen = new Set<string>();
    const dsNames = new Set((gc.datasources ?? []).map((d) => d.name));
    gc.layers.forEach((l, i) => validateLayer(l, `layers.${i}`, issues, seen, dsNames));
  }

  return issues;
}

function validateDatasource(
  ds: Datasource,
  path: string,
  issues: Issue[],
  seen: Set<string>
) {
  if (!ds.name)
    issues.push({ path: `${path}.name`, severity: 'error', message: 'datasource name is required' });
  else if (seen.has(ds.name))
    issues.push({ path: `${path}.name`, severity: 'error', message: `duplicate datasource name "${ds.name}"` });
  else seen.add(ds.name);

  if (!ds.type)
    issues.push({ path: `${path}.type`, severity: 'error', message: 'datasource type is required' });

  const conf = ds.conf ?? {};
  switch (ds.type) {
    case 'geojson':
      if (conf.data === undefined)
        issues.push({ path: `${path}.conf.data`, severity: 'error', message: 'inline geojson requires conf.data (FeatureCollection)' });
      break;
    case 'geojson+http+remote':
      if (!conf.source)
        issues.push({ path: `${path}.conf.source`, severity: 'error', message: 'remote geojson requires conf.source' });
      break;
    case 'csv':
      if (conf.data === undefined)
        issues.push({ path: `${path}.conf.data`, severity: 'error', message: 'inline csv requires conf.data' });
      validateCsvStructure(conf.structure as CsvColumn[] | undefined, `${path}.conf.structure`, issues);
      break;
    case 'csv+http+remote':
      if (!conf.source)
        issues.push({ path: `${path}.conf.source`, severity: 'error', message: 'remote csv requires conf.source' });
      validateCsvStructure(conf.structure as CsvColumn[] | undefined, `${path}.conf.structure`, issues);
      break;
    default:
      issues.push({ path: `${path}.type`, severity: 'warn', message: `unknown datasource type "${ds.type}"` });
  }
}

function validateCsvStructure(
  s: CsvColumn[] | undefined,
  path: string,
  issues: Issue[]
) {
  if (!Array.isArray(s) || s.length === 0) {
    issues.push({ path, severity: 'error', message: 'csv requires conf.structure[]' });
    return;
  }
  let hasLat = false;
  let hasLon = false;
  s.forEach((c, i) => {
    if (!c.column) issues.push({ path: `${path}.${i}.column`, severity: 'error', message: 'column name required' });
    if (!Array.isArray(c.tags)) {
      issues.push({ path: `${path}.${i}.tags`, severity: 'error', message: 'tags must be an array' });
      return;
    }
    if (c.tags.includes('gcx:lat')) hasLat = true;
    if (c.tags.includes('gcx:lon')) hasLon = true;
  });
  if (!hasLat) issues.push({ path, severity: 'error', message: 'csv structure must tag a column as "gcx:lat"' });
  if (!hasLon) issues.push({ path, severity: 'error', message: 'csv structure must tag a column as "gcx:lon"' });
}

function validateLayer(
  l: Layer,
  path: string,
  issues: Issue[],
  seen: Set<string>,
  dsNames: Set<string>
) {
  if (!l.name)
    issues.push({ path: `${path}.name`, severity: 'error', message: 'layer name is required' });
  else if (seen.has(l.name))
    issues.push({ path: `${path}.name`, severity: 'error', message: `duplicate layer name "${l.name}"` });
  else seen.add(l.name);

  if (!l.type)
    issues.push({ path: `${path}.type`, severity: 'error', message: 'layer type is required' });

  const dataDriven = ['features', 'feature', 'markers'];
  if (dataDriven.includes(l.type)) {
    if (!l.datasource)
      issues.push({ path: `${path}.datasource`, severity: 'error', message: `${l.type} layer requires a datasource` });
    else if (!dsNames.has(l.datasource))
      issues.push({ path: `${path}.datasource`, severity: 'error', message: `datasource "${l.datasource}" not declared` });
  }

  // style.maplibre — each entry must have `type`
  const ml = l.style?.maplibre;
  if (ml) {
    const arr = Array.isArray(ml) ? ml : [ml];
    arr.forEach((entry, i) => {
      if (!entry || typeof entry !== 'object' || typeof entry.type !== 'string')
        issues.push({
          path: `${path}.style.maplibre.${i}.type`,
          severity: 'error',
          message: 'each maplibre layer entry must include a string "type"'
        });
      if (entry && typeof entry === 'object' && ('id' in entry || 'source' in entry))
        issues.push({
          path: `${path}.style.maplibre.${i}`,
          severity: 'warn',
          message: 'id/source are set by the runtime — remove from style.maplibre'
        });
    });
  }

  // detail.media — kind + src required
  const media = l.detail?.media;
  if (media) {
    if (!Array.isArray(media))
      issues.push({ path: `${path}.detail.media`, severity: 'error', message: 'detail.media must be an array' });
    else
      media.forEach((m, i) => {
        if (!m.kind || !['image', 'html', 'download'].includes(m.kind))
          issues.push({ path: `${path}.detail.media.${i}.kind`, severity: 'error', message: 'kind must be image | html | download' });
        if (!m.src)
          issues.push({ path: `${path}.detail.media.${i}.src`, severity: 'error', message: 'src is required' });
      });
  }
}

export function summarize(issues: Issue[]): { errors: number; warns: number } {
  let errors = 0;
  let warns = 0;
  for (const i of issues) {
    if (i.severity === 'error') errors++;
    else warns++;
  }
  return { errors, warns };
}
