<script lang="ts">
  import { onMount } from 'svelte';
  import {
    isTauri,
    authState,
    authSetToken,
    authClearToken,
    authPatUrl,
    githubOauthStart,
    githubOauthPoll,
    githubWhoami,
    openExternal,
    type WhoAmI,
    type DeviceFlowStart
  } from '$lib/tauri';

  let { onchange }: { onchange?: () => void | Promise<void> } = $props();

  let open = $state(false);
  let signing = $state(false);
  let pasteOpen = $state(false);
  let pat = $state('');
  let device = $state<DeviceFlowStart | null>(null);
  let pollTimer = $state<number | null>(null);
  let me = $state<WhoAmI | null>(null);
  let hasToken = $state(false);
  let oauthAvailable = $state(false);
  let err = $state<string | null>(null);

  let trigger = $state<HTMLButtonElement | undefined>(undefined);
  let panel = $state<HTMLDivElement | undefined>(undefined);

  async function refresh(): Promise<void> {
    if (!isTauri()) return;
    try {
      const s = await authState();
      hasToken = s.has_token;
      oauthAvailable = s.oauth_available;
      if (s.has_token) {
        me = await githubWhoami();
      } else {
        me = null;
      }
      onchange?.();
    } catch (e) {
      err = (e as Error).message;
    }
  }

  function close() {
    open = false;
    cancelPoll();
    pasteOpen = false;
    err = null;
  }
  function toggle() {
    if (open) close();
    else { open = true; err = null; refresh(); }
  }

  function cancelPoll() {
    if (pollTimer !== null) { clearInterval(pollTimer); pollTimer = null; }
    device = null;
    signing = false;
  }

  async function startDeviceFlow() {
    err = null;
    signing = true;
    try {
      device = await githubOauthStart();
      openExternal(device.verification_uri);
      const interval = Math.max(2, device.interval);
      pollTimer = setInterval(poll, interval * 1000) as unknown as number;
      // also poll once immediately so the UI feels responsive
      poll();
    } catch (e) {
      err = (e as Error).message;
      signing = false;
    }
  }

  async function poll() {
    if (!device) return;
    try {
      const res = await githubOauthPoll(device.device_code);
      if (res.kind === 'token') {
        await authSetToken(res.access_token);
        cancelPoll();
        await refresh();
      } else if (res.kind === 'denied' || res.kind === 'expired') {
        err = res.kind === 'denied' ? 'Sign-in denied.' : 'Code expired — try again.';
        cancelPoll();
      } else if (res.kind === 'error') {
        err = res.message;
        cancelPoll();
      } else if (res.kind === 'slow_down') {
        // backoff: clear and reset at the larger interval
        if (pollTimer !== null) clearInterval(pollTimer);
        pollTimer = setInterval(poll, res.interval * 1000) as unknown as number;
      }
      // pending: keep polling
    } catch (e) {
      err = (e as Error).message;
      cancelPoll();
    }
  }

  async function openPatPage() {
    err = null;
    try {
      const url = await authPatUrl();
      openExternal(url);
      pasteOpen = true;
    } catch (e) { err = (e as Error).message; }
  }

  async function submitPat() {
    if (!pat.trim()) return;
    err = null;
    try {
      await authSetToken(pat.trim());
      pat = '';
      pasteOpen = false;
      await refresh();
    } catch (e) { err = (e as Error).message; }
  }

  async function signOut() {
    if (!confirm('Sign out and forget the stored GitHub token?')) return;
    try {
      await authClearToken();
      await refresh();
    } catch (e) { err = (e as Error).message; }
  }

  onMount(() => {
    refresh();
    const docMd = (e: MouseEvent) => {
      if (!open) return;
      const t = e.target as Node;
      if (panel && !panel.contains(t) && trigger && !trigger.contains(t)) close();
    };
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') close(); };
    document.addEventListener('mousedown', docMd);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', docMd);
      document.removeEventListener('keydown', onKey);
      cancelPoll();
    };
  });
</script>

<div class="acc">
  <button class="acc__trigger" bind:this={trigger} onclick={toggle} title="GitHub account">
    {#if me}
      <span class="acc__handle">@{me.login}</span>
    {:else}
      <span class="acc__handle acc__handle--muted">Sign in</span>
    {/if}
  </button>

  {#if open}
    <div class="acc__panel" bind:this={panel}>
      {#if me}
        <div class="acc__head">
          <div class="col" style="gap: 2px;">
            <span class="acc__name">{me.name ?? me.login}</span>
            <span class="meta mono">@{me.login}</span>
          </div>
          <button class="btn btn--danger" onclick={signOut}>Sign out</button>
        </div>
        {#if me.scopes.length > 0}
          <p class="meta" style="padding: 0 var(--s-3); margin: 0 0 var(--s-2);">
            Scopes: <span class="mono">{me.scopes.join(' · ')}</span>
          </p>
        {/if}
      {:else}
        <div class="acc__head col" style="gap: var(--s-2);">
          <span class="label" style="margin: 0;">Connect to GitHub</span>
          {#if !signing && !pasteOpen}
            <div class="col" style="gap: var(--s-2);">
              {#if oauthAvailable}
                <button class="btn btn--primary" onclick={startDeviceFlow}>Sign in via browser</button>
              {/if}
              <button class="btn" onclick={openPatPage}>Use a personal access token</button>
            </div>
            <p class="meta" style="margin: 0;">
              {#if oauthAvailable}
                The browser flow opens GitHub's device-authorisation page; the token never reaches a third party.
              {:else}
                Browser sign-in is unavailable in this build — paste a token instead.
              {/if}
            </p>
          {/if}

          {#if signing && device}
            <p class="meta" style="margin: 0;">
              In the browser, enter this one-time code:
            </p>
            <div class="acc__code mono">{device.user_code}</div>
            <p class="meta" style="margin: 0;">
              Page didn't open?
              <a href={device.verification_uri} onclick={(e) => { e.preventDefault(); openExternal(device!.verification_uri); }}>
                {device.verification_uri}
              </a>
            </p>
            <button class="btn" onclick={cancelPoll}>Cancel</button>
          {/if}

          {#if pasteOpen}
            <p class="meta" style="margin: 0;">
              The GitHub token page is open. Generate a token with the
              <span class="mono">repo</span> scope, then paste it here.
            </p>
            <label class="field">
              <span class="label">Token</span>
              <input
                type="password"
                autocomplete="off"
                bind:value={pat}
                placeholder="ghp_… or github_pat_…"
                onkeydown={(e) => { if (e.key === 'Enter') submitPat(); }} />
            </label>
            <div class="row" style="justify-content: flex-end; gap: var(--s-3);">
              <button class="btn" onclick={() => { pasteOpen = false; pat = ''; }}>Cancel</button>
              <button class="btn btn--primary" onclick={submitPat} disabled={!pat.trim()}>Save</button>
            </div>
          {/if}
        </div>
      {/if}

      {#if err}<p class="error" style="padding: 0 var(--s-3) var(--s-2);">{err}</p>{/if}
    </div>
  {/if}
</div>

<style>
  .acc { position: relative; display: inline-flex; align-items: baseline; }
  .acc__trigger {
    font: inherit;
    background: transparent;
    border: 0;
    color: var(--ink);
    padding: 0;
    cursor: pointer;
    transition: color var(--d-fast) var(--ease-out-quart);
  }
  .acc__trigger:hover { color: var(--accent); }
  .acc__handle {
    font-size: var(--t-sm);
    font-weight: var(--w-med);
    letter-spacing: var(--track-tight);
  }
  .acc__handle--muted { color: var(--ink-mute); font-weight: var(--w-reg); }

  .acc__panel {
    position: absolute;
    top: calc(100% + var(--s-3));
    right: 0;
    min-width: 320px;
    max-width: 420px;
    background: var(--bg-raised);
    border: var(--hairline) solid var(--rule);
    box-shadow: 0 12px 32px -16px oklch(0% 0 0 / 0.18);
    z-index: 40;
    display: flex;
    flex-direction: column;
  }
  .acc__head {
    padding: var(--s-3);
    border-bottom: var(--hairline) solid var(--rule);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--s-3);
  }
  .acc__name { font-size: var(--t-base); font-weight: var(--w-sem); }
  .acc__code {
    font-size: var(--t-xl);
    letter-spacing: var(--track-loose);
    text-align: center;
    padding: var(--s-3);
    background: var(--bg-inset);
    border: var(--hairline) solid var(--rule);
    user-select: all;
  }
</style>
