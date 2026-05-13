// Type definitions mirroring openhistorymap/geocontext-front FORMAT.md.
// The runtime is the source of truth — when in doubt, the front-end wins
// and we accept extra keys (see "unknown top-level keys are preserved").

export type LatLon = [number, number]; // [lat, lon] — note: lat first

export type BackgroundAlias = 'osm' | 'ofm' | 'none';

export type BackgroundValue =
  | BackgroundAlias
  | string // alias or HTTPS tile-url template
  | false
  | null
  | { type: string; conf?: Record<string, unknown>; style?: unknown };

export type DemValue =
  | string
  | false
  | null
  | {
      url: string;
      encoding?: 'terrarium' | 'mapbox';
      hillshade?: boolean;
      terrain?: boolean;
      exaggeration?: number;
    };

export type CsvColumnType = 'number' | 'string' | 'boolean';

export interface CsvColumn {
  column: string;
  type: CsvColumnType;
  tags: string[]; // e.g. ["gcx:lat","gcx:geo"], ["gcx:title"]
}

export type DatasourceType =
  | 'geojson'
  | 'geojson+http+remote'
  | 'csv'
  | 'csv+http+remote'
  | (string & {}); // open set — runtime registers more

export interface Datasource {
  name: string;
  type: DatasourceType;
  conf: {
    // geojson
    data?: unknown;
    // *+http+remote
    source?: string;
    // csv / csv+http+remote
    structure?: CsvColumn[];
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

export type LayerType =
  | 'features'
  | 'feature'
  | 'markers'
  | 'raster-tiled'
  | 'osm-tiled'
  | 'ofm-tiled'
  | 'raster-dem'
  | 'vector-tiles'
  | (string & {});

export type StyleMode = 'marker' | 'line' | 'polygon';

export interface StyleOptions {
  radius?: number;
  fillColor?: string;
  color?: string;
  weight?: number;
  opacity?: number;
  fillOpacity?: number;
  [key: string]: unknown;
}

// A raw MapLibre-GL style-spec layer (id / source set by runtime).
export interface MaplibreLayer {
  type: string; // required
  filter?: unknown;
  paint?: Record<string, unknown>;
  layout?: Record<string, unknown>;
  minzoom?: number;
  maxzoom?: number;
  [key: string]: unknown;
}

export interface StyleBlock {
  style?: string; // ignored / reserved
  mode?: StyleMode;
  markerType?: 'circle' | (string & {});
  options?: StyleOptions;
  maplibre?: MaplibreLayer | MaplibreLayer[];
  [key: string]: unknown;
}

export type MediaKind = 'image' | 'html' | 'download';

export interface MediaItem {
  kind: MediaKind;
  src: string;
  label?: string;
}

export interface DetailBlock {
  title?: string;
  media?: MediaItem[];
  [key: string]: unknown;
}

export interface Layer {
  name: string;
  type: LayerType;
  datasource?: string;
  style?: StyleBlock;
  detail?: DetailBlock;
  interactive?: boolean;
  conf?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface GeoContext {
  title: string;
  type: '2d' | (string & {});
  center: LatLon;
  minzoom: number;
  startzoom: number;
  maxzoom: number;
  background?: BackgroundValue;
  dem?: DemValue;
  search?: boolean;
  datasources: Datasource[];
  layers: Layer[];
  [key: string]: unknown;
}

// ---------------------------------------------------------------------------
// Factories — sensible defaults for the editor

export function emptyGeoContext(): GeoContext {
  return {
    title: 'Untitled',
    type: '2d',
    center: [0, 0],
    minzoom: 1,
    startzoom: 4,
    maxzoom: 20,
    background: 'osm',
    search: true,
    datasources: [],
    layers: []
  };
}

export function newDatasource(name = `ds_${Date.now().toString(36)}`): Datasource {
  return {
    name,
    type: 'geojson+http+remote',
    conf: { source: '' }
  };
}

export function newLayer(name = `layer_${Date.now().toString(36)}`): Layer {
  return {
    name,
    type: 'features',
    datasource: '',
    style: {
      mode: 'marker',
      markerType: 'circle',
      options: {
        radius: 4,
        fillColor: '#e77148',
        color: '#000000',
        weight: 1,
        opacity: 1,
        fillOpacity: 0.6
      }
    }
  };
}

export function newCsvColumn(): CsvColumn {
  return { column: '', type: 'string', tags: [] };
}

export function newMediaItem(): MediaItem {
  return { kind: 'image', src: '', label: '' };
}
