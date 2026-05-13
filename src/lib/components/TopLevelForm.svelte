<script lang="ts">
  import type { GeoContext, BackgroundValue } from '$lib/types';
  import { issuesByPath } from '$lib/issuesByPath';
  import type { Issue } from '$lib/validate';

  let { model = $bindable(), issues }: { model: GeoContext; issues: Issue[] } = $props();
  let pathIssues = $derived(issuesByPath(issues));

  // Background — three modes: alias / url / disabled (object & null collapse to UI 'none')
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
  let bgAlias = $derived(typeof model.background === 'string' && !/^https?:\/\//i.test(model.background) ? model.background : 'osm');
  let bgUrl = $derived(typeof model.background === 'string' && /^https?:\/\//i.test(model.background) ? model.background : '');

  function setBg(next: BackgroundValue) { model.background = next; }
  function setBgMode(m: 'alias' | 'url' | 'none' | 'object') {
    if (m === 'none') setBg(false);
    else if (m === 'alias') setBg(bgAlias || 'osm');
    else if (m === 'url') setBg(bgUrl || 'https://');
  }

  function err(path: string): string | null {
    const list = pathIssues.get(path);
    if (!list?.length) return null;
    return list.map((i) => i.message).join('; ');
  }
</script>

<div class="col" style="gap: 12px;">
  <div class="card col">
    <h3 style="margin: 0 0 4px 0;">Map</h3>
    <label class="field">title
      <input type="text" bind:value={model.title} />
      {#if err('title')}<span class="error">{err('title')}</span>{/if}
    </label>

    <label class="field">type
      <select bind:value={model.type}>
        <option value="2d">2d</option>
      </select>
    </label>

    <div class="row" style="gap: 12px;">
      <label class="field" style="flex: 1;">center.lat
        <input type="number" step="0.000001" value={model.center?.[0] ?? 0}
               oninput={(e) => model.center = [+(e.currentTarget as HTMLInputElement).value, model.center?.[1] ?? 0]} />
        {#if err('center.0')}<span class="error">{err('center.0')}</span>{/if}
      </label>
      <label class="field" style="flex: 1;">center.lon
        <input type="number" step="0.000001" value={model.center?.[1] ?? 0}
               oninput={(e) => model.center = [model.center?.[0] ?? 0, +(e.currentTarget as HTMLInputElement).value]} />
        {#if err('center.1')}<span class="error">{err('center.1')}</span>{/if}
      </label>
    </div>

    <div class="row" style="gap: 12px;">
      <label class="field" style="flex: 1;">minzoom
        <input type="number" min="0" max="24" bind:value={model.minzoom} />
        {#if err('minzoom')}<span class="error">{err('minzoom')}</span>{/if}
      </label>
      <label class="field" style="flex: 1;">startzoom
        <input type="number" min="0" max="24" bind:value={model.startzoom} />
        {#if err('startzoom')}<span class="error">{err('startzoom')}</span>{/if}
      </label>
      <label class="field" style="flex: 1;">maxzoom
        <input type="number" min="0" max="24" bind:value={model.maxzoom} />
        {#if err('maxzoom')}<span class="error">{err('maxzoom')}</span>{/if}
      </label>
    </div>

    <label class="field row" style="flex-direction: row; align-items: center; gap: 6px;">
      <input type="checkbox" checked={model.search !== false}
             onchange={(e) => model.search = (e.currentTarget as HTMLInputElement).checked} />
      <span style="color: var(--fg);">enable search UI</span>
    </label>
  </div>

  <div class="card col">
    <h3 style="margin: 0 0 4px 0;">Background</h3>
    <div class="row">
      <label class="field" style="flex: 1;">mode
        <select value={bgMode} onchange={(e) => setBgMode((e.currentTarget as HTMLSelectElement).value as BgMode)}>
          <option value="alias">alias (osm / ofm)</option>
          <option value="url">raw XYZ URL</option>
          <option value="none">none</option>
          <option value="object">object (advanced)</option>
        </select>
      </label>
      {#if bgMode === 'alias'}
        <label class="field" style="flex: 1;">alias
          <select value={bgAlias} onchange={(e) => setBg((e.currentTarget as HTMLSelectElement).value)}>
            <option value="osm">osm</option>
            <option value="ofm">ofm</option>
          </select>
        </label>
      {:else if bgMode === 'url'}
        <label class="field" style="flex: 2;">tile template
          <input type="text" placeholder="https://example.org/{'{z}'}/{'{x}'}/{'{y}'}.png"
                 value={bgUrl}
                 oninput={(e) => setBg((e.currentTarget as HTMLInputElement).value)} />
        </label>
      {:else if bgMode === 'object'}
        <label class="field" style="flex: 2;">JSON
          <textarea rows="3"
                    value={JSON.stringify(model.background ?? {}, null, 2)}
                    onchange={(e) => {
                      try { setBg(JSON.parse((e.currentTarget as HTMLTextAreaElement).value)); } catch {}
                    }}></textarea>
        </label>
      {/if}
    </div>
  </div>

  <div class="card col">
    <h3 style="margin: 0 0 4px 0;">Terrain / hillshade <span class="muted">(MapLibre flavour)</span></h3>
    <label class="field">DEM tile URL
      <input type="text" placeholder="https://… /{'{z}'}/{'{x}'}/{'{y}'}.png — empty = no DEM"
             value={typeof model.dem === 'string' ? model.dem : (model.dem && typeof model.dem === 'object' ? (model.dem.url ?? '') : '')}
             oninput={(e) => {
               const v = (e.currentTarget as HTMLInputElement).value;
               if (!v) { delete (model as any).dem; return; }
               if (typeof model.dem === 'object' && model.dem) (model.dem as any).url = v;
               else model.dem = v;
             }} />
    </label>
  </div>
</div>
