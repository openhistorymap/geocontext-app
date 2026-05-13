<script lang="ts">
  import { pickFolder, gitClone, type CloneResult } from '$lib/tauri';

  let {
    onclose,
    oncloned
  }: {
    onclose: () => void;
    oncloned: (result: CloneResult) => void | Promise<void>;
  } = $props();

  let url = $state('');
  let dest = $state('');
  let folderName = $state('');
  let busy = $state(false);
  let err = $state<string | null>(null);

  $effect(() => {
    if (!url) { folderName = ''; return; }
    const m = url.trim().match(/([^\/]+?)(?:\.git)?\/?$/);
    folderName = m ? m[1] : '';
  });

  async function pickDest() {
    const f = await pickFolder();
    if (f) dest = f;
  }

  async function run() {
    if (!url.trim() || !dest.trim()) {
      err = 'Both the repo URL and a destination folder are required.';
      return;
    }
    busy = true; err = null;
    try {
      const sep = dest.includes('\\') ? '\\' : '/';
      const target = folderName
        ? (dest.endsWith(sep) ? dest + folderName : dest + sep + folderName)
        : dest;
      const result = await gitClone(url.trim(), target);
      await oncloned(result);
      onclose();
    } catch (e) {
      err = (e as Error).message;
    } finally {
      busy = false;
    }
  }
</script>

<div class="cd__back" role="presentation" onclick={onclose}></div>
<div class="cd" role="dialog" aria-modal="true" aria-label="Clone from GitHub">
  <div class="cd__head">
    <span class="section__title">Clone from GitHub</span>
    <button class="btn btn--icon" title="close" onclick={onclose}>✕</button>
  </div>
  <div class="col" style="padding: var(--s-3); gap: var(--s-3);">
    <label class="field">
      <span class="label">Repository</span>
      <input
        type="text"
        bind:value={url}
        placeholder="owner/repo or https://github.com/owner/repo"
        autocomplete="off" />
    </label>
    <div class="row" style="gap: var(--s-3); align-items: flex-end;">
      <label class="field" style="flex: 1; min-width: 220px;">
        <span class="label">Destination folder</span>
        <input type="text" bind:value={dest} placeholder="/Users/you/maps" />
      </label>
      <button class="btn" onclick={pickDest}>Browse…</button>
    </div>
    {#if folderName}
      <p class="meta">
        Will clone into <span class="mono">{dest}/{folderName}</span>
      </p>
    {/if}
    {#if err}<p class="error">{err}</p>{/if}
    <div class="row" style="justify-content: flex-end; gap: var(--s-3); margin-top: var(--s-2);">
      <button class="btn" onclick={onclose} disabled={busy}>Cancel</button>
      <button class="btn btn--primary" onclick={run} disabled={busy || !url || !dest}>
        {busy ? 'Cloning…' : 'Clone'}
      </button>
    </div>
  </div>
</div>

<style>
  .cd__back {
    position: fixed; inset: 0;
    background: oklch(0% 0 0 / 0.25);
    z-index: 60;
  }
  .cd {
    position: fixed;
    top: 12vh;
    left: 50%;
    transform: translateX(-50%);
    width: min(560px, 92vw);
    background: var(--bg-raised);
    border: var(--hairline) solid var(--rule);
    box-shadow: 0 24px 64px -32px oklch(0% 0 0 / 0.35);
    z-index: 61;
  }
  .cd__head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--s-3);
    border-bottom: var(--hairline) solid var(--rule);
  }
</style>
