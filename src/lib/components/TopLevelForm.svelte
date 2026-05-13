<script lang="ts">
  import type { GeoContext, BackgroundValue } from '$lib/types';
  import { issuesByPath } from '$lib/issuesByPath';
  import type { Issue } from '$lib/validate';

  let { model = $bindable(), issues }: { model: GeoContext; issues: Issue[] } = $props();
  let pathIssues = $derived(issuesByPath(issues));

  type BgMode = 'alias' | 'url' | 'none' | 'object';
  let bgMode: BgMode = $derived.by(() => {
    const b = model.background;
    if (b === false || b === null || b === 'none') return 'none';
    if (typeof b === 'string') {
      if (/^https?:\/\//i.test(b)) return 'url';
      return 'alias';
    }
    if (typeof b === 'object') return 'object';
    return 'alias';
  });
  let bgAlias = $derived(
    typeof model.background === 'string' && !/^https?:\/\//i.test(model.background)
      ? model.background
      : 'osm'
  );
  let bgUrl = $derived(
    typeof model.background === 'string' && /^https?:\/\//i.test(model.background) ? model.background : ''
  );

  function setBg(next: BackgroundValue) { model.background = next; }
  function setBgMode(m: BgMode) {
    if (m === 'none') setBg(false);
    else if (m === 'alias') setBg(bgAlias || 'osm');
    else if (m === 'url') setBg(bgUrl || 'https://');
  }

  function err(path: string): string | null {
    const list = pathIssues.get(path);
    if (!list?.length) return null;
    return list.map((i) => i.message).join('; ');
  }

  function demUrl(): string {
    if (typeof model.dem === 'string') return model.dem;
    if (model.dem && typeof model.dem === 'object') return model.dem.url ?? '';
    return '';
  }
  function setDemUrl(v: string) {
    if (!v) { delete (model as any).dem; return; }
    if (typeof model.dem === 'object' && model.dem) (model.dem as any).url = v;
    else model.dem = v;
  }
</script>

<div class="stack">
  <div class="section">
    <div class="section__head">
      <span class="section__title">Map</span>
      <span class="section__hint">Title, projection-anchor, zoom bounds.</span>
    </div>

    <label class="field">
      <span class="label">Title</span>
      <input type="text" bind:value={model.title} placeholder="e.g. Valle Trebba" />
      {#if err('title')}<span class="error">{err('title')}</span>{/if}
    </label>

    <label class="field">
      <span class="label">Type</span>
      <select bind:value={model.type}>
        <option value="2d">2d</option>
      </select>
    </label>

    <div class="row wrap" style="gap: var(--s-5);">
      <label class="field" style="flex: 1; min-width: 180px;">
        <span class="label">Centre — Latitude</span>
        <input
          type="number"
          step="0.000001"
          value={model.center?.[0] ?? 0}
          oninput={(e) =>
            (model.center = [
              +(e.currentTarget as HTMLInputElement).value,
              model.center?.[1] ?? 0
            ])} />
        {#if err('center.0')}<span class="error">{err('center.0')}</span>{/if}
      </label>
      <label class="field" style="flex: 1; min-width: 180px;">
        <span class="label">Centre — Longitude</span>
        <input
          type="number"
          step="0.000001"
          value={model.center?.[1] ?? 0}
          oninput={(e) =>
            (model.center = [
              model.center?.[0] ?? 0,
              +(e.currentTarget as HTMLInputElement).value
            ])} />
        {#if err('center.1')}<span class="error">{err('center.1')}</span>{/if}
      </label>
    </div>

    <div class="row wrap" style="gap: var(--s-5);">
      <label class="field" style="flex: 1; min-width: 100px;">
        <span class="label">Min zoom</span>
        <input type="number" min="0" max="24" bind:value={model.minzoom} />
        {#if err('minzoom')}<span class="error">{err('minzoom')}</span>{/if}
      </label>
      <label class="field" style="flex: 1; min-width: 100px;">
        <span class="label">Start zoom</span>
        <input type="number" min="0" max="24" bind:value={model.startzoom} />
        {#if err('startzoom')}<span class="warn">{err('startzoom')}</span>{/if}
      </label>
      <label class="field" style="flex: 1; min-width: 100px;">
        <span class="label">Max zoom</span>
        <input type="number" min="0" max="24" bind:value={model.maxzoom} />
        {#if err('maxzoom')}<span class="error">{err('maxzoom')}</span>{/if}
      </label>
    </div>

    <label class="row" style="gap: var(--s-2);">
      <input
        type="checkbox"
        checked={model.search !== false}
        onchange={(e) => (model.search = (e.currentTarget as HTMLInputElement).checked)} />
      <span class="meta" style="color: var(--ink);">Enable search UI</span>
    </label>
  </div>

  <div class="section">
    <div class="section__head">
      <span class="section__title">Background</span>
      <span class="section__hint">Basemap tiles drawn beneath user layers.</span>
    </div>
    <div class="row wrap" style="gap: var(--s-5);">
      <label class="field" style="flex: 1; min-width: 160px;">
        <span class="label">Mode</span>
        <select
          value={bgMode}
          onchange={(e) =>
            setBgMode((e.currentTarget as HTMLSelectElement).value as BgMode)}>
          <option value="alias">Alias (osm / ofm)</option>
          <option value="url">Raw XYZ template</option>
          <option value="none">No basemap</option>
          <option value="object">Object (advanced)</option>
        </select>
      </label>

      {#if bgMode === 'alias'}
        <label class="field" style="flex: 1; min-width: 140px;">
          <span class="label">Alias</span>
          <select
            value={bgAlias}
            onchange={(e) => setBg((e.currentTarget as HTMLSelectElement).value)}>
            <option value="osm">osm — OpenStreetMap</option>
            <option value="ofm">ofm — OpenFreeMap</option>
          </select>
        </label>
      {:else if bgMode === 'url'}
        <label class="field" style="flex: 2; min-width: 260px;">
          <span class="label">Tile template</span>
          <input
            type="text"
            placeholder={"https://example.org/{z}/{x}/{y}.png"}
            value={bgUrl}
            oninput={(e) => setBg((e.currentTarget as HTMLInputElement).value)} />
        </label>
      {:else if bgMode === 'object'}
        <label class="field" style="flex: 2; min-width: 260px;">
          <span class="label">JSON</span>
          <textarea
            rows="3"
            value={JSON.stringify(model.background ?? {}, null, 2)}
            onchange={(e) => {
              try {
                setBg(JSON.parse((e.currentTarget as HTMLTextAreaElement).value));
              } catch {}
            }}></textarea>
        </label>
      {/if}
    </div>
  </div>

  <div class="section">
    <div class="section__head">
      <span class="section__title">Terrain</span>
      <span class="section__hint">DEM source for hillshade — MapLibre flavour only.</span>
    </div>
    <label class="field">
      <span class="label">DEM tile URL</span>
      <input
        type="text"
        placeholder={"https://… /{z}/{x}/{y}.png   — empty disables"}
        value={demUrl()}
        oninput={(e) => setDemUrl((e.currentTarget as HTMLInputElement).value)} />
    </label>
  </div>
</div>
