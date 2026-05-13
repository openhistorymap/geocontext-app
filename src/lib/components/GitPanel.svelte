<script lang="ts">
  import { onMount } from 'svelte';
  import {
    isTauri,
    gitStatus,
    gitPull,
    gitSync,
    gitInitWithOrigin,
    githubCreateRepo,
    type GitStatus
  } from '$lib/tauri';

  let {
    folder,
    title,
    hasToken,
    onsynced
  }: {
    folder: string | null;
    title: string;
    hasToken: boolean;
    onsynced: (status: GitStatus) => void | Promise<void>;
  } = $props();

  let status = $state<GitStatus | null>(null);
  let loading = $state(false);
  let err = $state<string | null>(null);
  let busy = $state(false);

  // Sync flow state
  let syncOpen = $state(false);
  let syncMessage = $state('');

  // Publish flow state
  let publishOpen = $state(false);
  let publishName = $state('');
  let publishDescription = $state('');
  let publishOrg = $state('');
  let publishPrivate = $state(false);

  async function refresh() {
    if (!folder || !isTauri()) { status = null; return; }
    loading = true;
    err = null;
    try { status = await gitStatus(folder); }
    catch (e) { err = (e as Error).message; }
    finally { loading = false; }
  }

  $effect(() => { void folder; refresh(); });

  async function doPull() {
    if (!folder) return;
    busy = true; err = null;
    try {
      status = await gitPull(folder);
      onsynced?.(status);
    } catch (e) { err = (e as Error).message; }
    finally { busy = false; }
  }

  function openSyncForm() {
    if (!status) return;
    syncMessage =
      status.dirty > 0
        ? `Update ${title || 'geocontext'} (${status.dirty} file${status.dirty === 1 ? '' : 's'})`
        : 'Push pending commits';
    syncOpen = true;
  }

  async function doSync() {
    if (!folder) return;
    busy = true; err = null;
    try {
      status = await gitSync(folder, syncMessage || `Update ${title || 'geocontext'}`);
      syncOpen = false;
      onsynced?.(status);
    } catch (e) { err = (e as Error).message; }
    finally { busy = false; }
  }

  function openPublishForm() {
    publishName = (title || folder?.split(/[\\/]/).pop() || 'geocontext-map')
      .toLowerCase()
      .replace(/[^a-z0-9-]+/g, '-')
      .replace(/^-+|-+$/g, '');
    publishDescription = '';
    publishOrg = '';
    publishPrivate = false;
    publishOpen = true;
  }

  async function doPublish() {
    if (!folder) return;
    busy = true; err = null;
    try {
      const repo = await githubCreateRepo(
        publishName,
        publishDescription || null,
        publishPrivate,
        publishOrg.trim() ? publishOrg.trim() : null
      );
      await gitInitWithOrigin(folder, repo.clone_url);
      const first = await gitSync(folder, `Initial commit — ${title || publishName}`);
      status = first;
      publishOpen = false;
      onsynced?.(status);
    } catch (e) { err = (e as Error).message; }
    finally { busy = false; }
  }
</script>

<div class="gp">
  <div class="gp__line">
    {#if !folder}
      <span class="meta">No workspace open</span>
    {:else if !isTauri()}
      <span class="meta">Git available in the Tauri app only</span>
    {:else if loading}
      <span class="meta">Reading git state…</span>
    {:else if !status}
      <span class="meta">{err ?? '—'}</span>
    {:else if !status.is_repo}
      <span class="meta">Local folder is not a git repository.</span>
      <button class="btn" onclick={openPublishForm} disabled={!hasToken || busy}>Publish to GitHub…</button>
      {#if !hasToken}<span class="meta">Sign in first to enable publishing.</span>{/if}
    {:else}
      <span class="gp__branch mono">{status.branch ?? '(detached)'}</span>
      {#if status.remote_url}
        <span class="gp__remote mono" title={status.remote_url}>
          {status.remote_url.replace(/^https?:\/\//, '').replace(/\.git$/, '')}
        </span>
      {/if}
      <span class="gp__counts">
        <span class:gp__count--dim={status.ahead === 0}>↑ {status.ahead}</span>
        <span class:gp__count--dim={status.behind === 0}>↓ {status.behind}</span>
        <span class:gp__count--dim={status.dirty === 0}>● {status.dirty}</span>
      </span>
      <div class="row" style="gap: var(--s-3);">
        {#if status.remote_url}
          <button class="btn" onclick={doPull} disabled={busy || !hasToken}>Pull</button>
          <button class="btn btn--primary" onclick={openSyncForm} disabled={busy || !hasToken}>Sync</button>
        {:else}
          <button class="btn" onclick={openPublishForm} disabled={!hasToken || busy}>Publish to GitHub…</button>
        {/if}
      </div>
    {/if}
  </div>

  {#if err}<p class="error" style="margin: var(--s-2) 0 0;">{err}</p>{/if}

  {#if syncOpen}
    <div class="gp__form">
      <label class="field">
        <span class="label">Commit message</span>
        <input type="text" bind:value={syncMessage} placeholder="Update the map" />
      </label>
      <div class="row" style="justify-content: flex-end; gap: var(--s-3); margin-top: var(--s-2);">
        <button class="btn" onclick={() => (syncOpen = false)} disabled={busy}>Cancel</button>
        <button class="btn btn--primary" onclick={doSync} disabled={busy || !syncMessage.trim()}>
          {busy ? 'Syncing…' : 'Commit & push'}
        </button>
      </div>
    </div>
  {/if}

  {#if publishOpen}
    <div class="gp__form">
      <div class="row wrap" style="gap: var(--s-4);">
        <label class="field" style="flex: 2; min-width: 180px;">
          <span class="label">Repository name</span>
          <input type="text" bind:value={publishName} placeholder="valle-trebba" />
        </label>
        <label class="field" style="flex: 1; min-width: 140px;">
          <span class="label">Owner (org or empty = you)</span>
          <input type="text" bind:value={publishOrg} placeholder="openhistorymap" />
        </label>
      </div>
      <label class="field" style="margin-top: var(--s-3);">
        <span class="label">Description</span>
        <input type="text" bind:value={publishDescription} placeholder="One-line description" />
      </label>
      <label class="row" style="gap: var(--s-2); margin-top: var(--s-3);">
        <input type="checkbox" bind:checked={publishPrivate} />
        <span class="meta" style="color: var(--ink);">Private repository</span>
      </label>
      <p class="meta" style="margin-top: var(--s-2);">
        Creates the repo on GitHub, initialises this folder as a git repo, and pushes a first commit.
      </p>
      <div class="row" style="justify-content: flex-end; gap: var(--s-3); margin-top: var(--s-2);">
        <button class="btn" onclick={() => (publishOpen = false)} disabled={busy}>Cancel</button>
        <button class="btn btn--primary" onclick={doPublish} disabled={busy || !publishName.trim()}>
          {busy ? 'Publishing…' : 'Create & push'}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .gp {
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
  }
  .gp__line {
    display: flex;
    align-items: center;
    gap: var(--s-4);
    flex-wrap: wrap;
  }
  .gp__branch {
    font-size: var(--t-sm);
    font-weight: var(--w-med);
    color: var(--ink);
  }
  .gp__remote {
    font-size: var(--t-xs);
    color: var(--ink-mute);
  }
  .gp__counts {
    display: flex;
    gap: var(--s-3);
    font-family: var(--font-mono);
    font-size: var(--t-xs);
    color: var(--ink);
    font-variant-numeric: tabular-nums;
  }
  .gp__count--dim { color: var(--ink-mute); }

  .gp__form {
    border-top: var(--hairline) solid var(--rule-soft);
    padding-top: var(--s-3);
    margin-top: var(--s-2);
  }
</style>
