<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import maplibregl, { type Map as MlMap, type StyleSpecification } from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import type { GeoContext, Layer, Datasource, MaplibreLayer } from '$lib/types';
  import { resolveAssetPath, type RepoCoords } from '$lib/assetPath';

  let {
    model,
    repo,
    repoUser = $bindable(''),
    repoProject = $bindable(''),
    repoRef = $bindable('HEAD')
  }: {
    model: GeoContext;
    repo: RepoCoords | null;
    repoUser?: string;
    repoProject?: string;
    repoRef?: string;
  } = $props();

  let container: HTMLDivElement;
  let map: MlMap | null = null;
  let lastErr = $state<string | null>(null);

  function backgroundSource(): { source?: any; layer?: any } {
    const bg = model.background;
    if (bg === false || bg === null || bg === 'none') return {};
    if (typeof bg === 'string') {
      const url =
        bg === 'osm'
          ? 'https://tile.openstreetmap.org/{z}/{x}/{y}.png'
          : bg === 'ofm'
            ? 'https://tiles.openfreemap.org/styles/positron/{z}/{x}/{y}.png'
            : bg;
      return {
        source: { type: 'raster', tiles: [url], tileSize: 256, attribution: '© OpenStreetMap / OFM' },
        layer: { id: '__bg', type: 'raster', source: '__bg' }
      };
    }
    if (typeof bg === 'object' && bg && (bg as any).conf?.url) {
      return {
        source: { type: 'raster', tiles: [(bg as any).conf.url], tileSize: 256 },
        layer: { id: '__bg', type: 'raster', source: '__bg' }
      };
    }
    return {};
  }

  function buildStyle(): StyleSpecification {
    const sources: Record<string, any> = {};
    const layers: any[] = [];

    const bg = backgroundSource();
    if (bg.source && bg.layer) {
      sources.__bg = bg.source;
      layers.push(bg.layer);
    }

    // FORMAT §4 says layers[0] is drawn on top. MapLibre paints in array
    // order (later = on top), so we iterate in reverse.
    const ordered = [...(model.layers ?? [])].reverse();
    const dsByName = new Map<string, Datasource>();
    for (const d of model.datasources ?? []) dsByName.set(d.name, d);

    for (const lyr of ordered) {
      try {
        wireLayer(lyr, dsByName, sources, layers);
      } catch (e) {
        // surface but don't fail the whole style
        console.warn('preview: layer', lyr.name, 'skipped:', e);
      }
    }

    return {
      version: 8,
      glyphs: 'https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf',
      sources,
      layers
    } as StyleSpecification;
  }

  function wireLayer(
    lyr: Layer,
    dsByName: Map<string, Datasource>,
    sources: Record<string, any>,
    layers: any[]
  ) {
    if (lyr.type === 'osm-tiled' || lyr.type === 'raster-tiled' || lyr.type === 'ofm-tiled') {
      const url = (lyr.conf?.url as string) ?? 'https://tile.openstreetmap.org/{z}/{x}/{y}.png';
      const id = `src_${lyr.name}`;
      sources[id] = { type: 'raster', tiles: [url], tileSize: 256 };
      layers.push({ id: `lyr_${lyr.name}`, type: 'raster', source: id });
      return;
    }

    if (lyr.type === 'vector-tiles') {
      const url = lyr.conf?.url as string;
      if (!url) return;
      const id = `src_${lyr.name}`;
      sources[id] = { type: 'vector', tiles: [url] };
      // Without a known source-layer we can't render anything; require user to use style.maplibre
      return;
    }

    if (lyr.type !== 'features' && lyr.type !== 'feature' && lyr.type !== 'markers') return;
    if (!lyr.datasource) return;
    const ds = dsByName.get(lyr.datasource);
    if (!ds) return;

    const srcId = `src_${lyr.name}`;

    // Build GeoJSON source
    if (ds.type === 'geojson') {
      sources[srcId] = { type: 'geojson', data: ds.conf.data ?? { type: 'FeatureCollection', features: [] } };
    } else if (ds.type === 'geojson+http+remote') {
      const url = resolveAssetPath(String(ds.conf.source ?? ''), repo);
      if (!url) return;
      sources[srcId] = { type: 'geojson', data: url };
    } else if (ds.type === 'csv' || ds.type === 'csv+http+remote') {
      // CSV preview not implemented inline; show empty source so map still renders
      sources[srcId] = { type: 'geojson', data: { type: 'FeatureCollection', features: [] } };
    } else {
      return;
    }

    // Style: prefer raw style.maplibre when present
    const raw = lyr.style?.maplibre;
    if (raw) {
      const arr: MaplibreLayer[] = Array.isArray(raw) ? raw : [raw];
      arr.forEach((entry, i) => {
        if (!entry || typeof entry.type !== 'string') return;
        layers.push({ ...entry, id: `lyr_${lyr.name}_${i}`, source: srcId });
      });
      return;
    }

    // Auto-synthesize circle/line/fill triple, gated by geometry-type
    const opt = lyr.style?.options ?? {};
    const fillColor = (opt.fillColor as string) ?? '#e77148';
    const color = (opt.color as string) ?? '#000000';
    const radius = (opt.radius as number) ?? 4;
    const weight = (opt.weight as number) ?? 1;
    const fillOpacity = (opt.fillOpacity as number) ?? 0.6;
    const opacity = (opt.opacity as number) ?? 1;

    layers.push({
      id: `lyr_${lyr.name}_pt`,
      type: 'circle',
      source: srcId,
      filter: ['==', ['geometry-type'], 'Point'],
      paint: {
        'circle-radius': radius,
        'circle-color': fillColor,
        'circle-opacity': fillOpacity,
        'circle-stroke-color': color,
        'circle-stroke-width': weight,
        'circle-stroke-opacity': opacity
      }
    });
    layers.push({
      id: `lyr_${lyr.name}_ln`,
      type: 'line',
      source: srcId,
      filter: ['in', ['geometry-type'], ['literal', ['LineString', 'MultiLineString']]],
      paint: {
        'line-color': color,
        'line-width': weight,
        'line-opacity': opacity
      }
    });
    layers.push({
      id: `lyr_${lyr.name}_fl`,
      type: 'fill',
      source: srcId,
      filter: ['in', ['geometry-type'], ['literal', ['Polygon', 'MultiPolygon']]],
      paint: {
        'fill-color': fillColor,
        'fill-opacity': fillOpacity,
        'fill-outline-color': color
      }
    });
  }

  function applyStyle() {
    if (!map) return;
    try {
      map.setStyle(buildStyle());
      lastErr = null;
    } catch (e) {
      lastErr = (e as Error).message;
    }
  }

  function applyView() {
    if (!map || !model.center) return;
    const [lat, lon] = model.center;
    map.jumpTo({ center: [lon, lat], zoom: model.startzoom ?? 4 });
  }

  onMount(() => {
    map = new maplibregl.Map({
      container,
      style: buildStyle(),
      center: [model.center?.[1] ?? 0, model.center?.[0] ?? 0],
      zoom: model.startzoom ?? 4,
      minZoom: model.minzoom ?? 0,
      maxZoom: model.maxzoom ?? 22,
      attributionControl: { compact: true }
    });
    map.addControl(new maplibregl.NavigationControl(), 'top-right');
  });

  onDestroy(() => map?.remove());

  // Reactive updates: any change to the model rebuilds the style
  $effect(() => {
    void model;
    void repo;
    if (map) applyStyle();
  });

  // Re-centre when center/zoom edits land
  $effect(() => {
    if (map && model.center) {
      try {
        map.setMinZoom(model.minzoom ?? 0);
        map.setMaxZoom(model.maxzoom ?? 22);
      } catch {}
    }
  });
</script>

<div class="mp">
  <div class="mp__head">
    <div class="col" style="gap: 2px;">
      <span class="section__title">Preview</span>
      <span class="section__hint">Live MapLibre render of the working document.</span>
    </div>
    <div class="row" style="gap: var(--s-3);">
      <button class="btn" onclick={applyView}>Recentre</button>
      <button class="btn" onclick={applyStyle}>Refresh</button>
    </div>
  </div>

  {#if lastErr}<span class="error">{lastErr}</span>{/if}

  <div bind:this={container} class="mp__canvas"></div>

  <div class="mp__coords">
    <span class="label" style="margin: 0;">Repo coords</span>
    <input class="mp__inp" type="text" placeholder="user" bind:value={repoUser} />
    <span class="mp__sep">/</span>
    <input class="mp__inp" type="text" placeholder="project" bind:value={repoProject} />
    <span class="mp__sep">@</span>
    <input class="mp__inp mp__inp--narrow" type="text" placeholder="HEAD" bind:value={repoRef} />
  </div>

  <p class="meta" style="margin: 0;">
    Bare-relative paths resolve through jsDelivr when repo coords are set
    (§9). Vector-tile sources only render through the raw <span class="mono">style.maplibre</span> escape hatch.
  </p>
</div>

<style>
  .mp {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    height: 100%;
    min-height: 0;
  }
  .mp__head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: var(--s-3);
  }
  .mp__canvas {
    flex: 1;
    min-height: 240px;
    border: var(--hairline) solid var(--rule);
    background: var(--bg-raised);
    overflow: hidden;
  }
  .mp__coords {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    border-top: var(--hairline) solid var(--rule-soft);
    padding-top: var(--s-2);
    flex-wrap: wrap;
  }
  .mp__inp {
    font: inherit;
    font-family: var(--font-mono);
    font-size: var(--t-sm);
    color: var(--ink);
    background: transparent;
    border: 0;
    border-bottom: var(--hairline) solid var(--rule);
    padding: 2px 0;
    width: 110px;
  }
  .mp__inp--narrow { width: 70px; }
  .mp__inp:focus { outline: none; border-color: var(--accent); }
  .mp__sep { color: var(--ink-mute); font-family: var(--font-mono); font-size: var(--t-sm); }
</style>
