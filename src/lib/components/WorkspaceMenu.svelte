<script lang="ts">
  import { onMount } from 'svelte';
  import {
    isTauri,
    listWorkspaces,
    removeWorkspace,
    forgetUnreachableWorkspaces,
    type WorkspaceEntry
  } from '$lib/tauri';

  let {
    activePath,
    onpick,
    onnew,
    onopenfolder
  }: {
    activePath: string | null;
    onpick: (entry: WorkspaceEntry) => void | Promise<void>;
    onnew: () => void | Promise<void>;
    onopenfolder: () => void | Promise<void>;
  } = $props();

  let open = $state(false);
  let workspaces = $state<WorkspaceEntry[]>([]);
  let err = $state<string | null>(null);
  let trigger = $state<HTMLButtonElement | undefined>(undefined);
  let panel = $state<HTMLDivElement | undefined>(undefined);

  export async function refresh(): Promise<void> {
    if (!isTauri()) { workspaces = []; return; }
    try {
      workspaces = await listWorkspaces();
    } catch (e) {
      err = (e as Error).message;
    }
  }

  let activeEntry = $derived(workspaces.find((w) => w.path === activePath) ?? null);

  function basename(p: string): string {
    return p.split(/[\\/]/).pop() ?? p;
  }

  function fmtAgo(unix: number): string {
    if (!unix) return '';
    const diff = Math.max(0, Date.now() / 1000 - unix);
    if (diff < 60) return 'just now';
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    if (diff < 86400 * 30) return `${Math.floor(diff / 86400)}d ago`;
    const d = new Date(unix * 1000);
    return d.toISOString().slice(0, 10);
  }

  async function toggle() {
    open = !open;
    if (open) await refresh();
  }

  function close() { open = false; }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  async function pick(entry: WorkspaceEntry) {
    if (!entry.reachable) return;
    close();
    await onpick(entry);
  }

  async function newRepo() {
    close();
    await onnew();
  }

  async function openFolder() {
    close();
    await onopenfolder();
  }

  async function removeOne(e: Event, entry: WorkspaceEntry) {
    e.stopPropagation();
    if (!confirm(`Remove ${entry.path} from the workspace list? (does not delete the folder)`)) return;
    try {
      workspaces = await removeWorkspace(entry.path);
    } catch (err2) {
      err = (err2 as Error).message;
    }
  }

  async function forgetUnreachable() {
    try {
      workspaces = await forgetUnreachableWorkspaces();
    } catch (e) {
      err = (e as Error).message;
    }
  }

  onMount(() => {
    refresh();
    const onDocClick = (e: MouseEvent) => {
      if (!open) return;
      const t = e.target as Node;
      if (panel && !panel.contains(t) && trigger && !trigger.contains(t)) close();
    };
    document.addEventListener('mousedown', onDocClick);
    document.addEventListener('keydown', handleKeydown);
    return () => {
      document.removeEventListener('mousedown', onDocClick);
      document.removeEventListener('keydown', handleKeydown);
    };
  });

  let unreachableCount = $derived(workspaces.filter((w) => !w.reachable).length);
</script>

<div class="wsm">
  <button
    bind:this={trigger}
    class="wsm__trigger"
    onclick={toggle}
    aria-haspopup="menu"
    aria-expanded={open}
    title="Workspaces"
  >
    {#if activeEntry}
      <span class="wsm__title">{activeEntry.title ?? basename(activeEntry.path)}</span>
    {:else if activePath}
      <span class="wsm__title">{basename(activePath)}</span>
    {:else}
      <span class="wsm__title wsm__title--muted">Select workspace</span>
    {/if}
    <span class="wsm__chev" class:wsm__chev--open={open}>▾</span>
  </button>

  {#if open}
    <div bind:this={panel} class="wsm__panel" role="menu">
      <div class="wsm__actions">
        <button class="btn" onclick={openFolder} disabled={!isTauri()}>Open folder…</button>
        <button class="btn" onclick={newRepo} disabled={!isTauri()}>New repository…</button>
      </div>

      {#if err}<p class="error" style="padding: 0 var(--s-3);">{err}</p>{/if}

      {#if workspaces.length === 0}
        <p class="meta" style="padding: var(--s-3);">
          No workspaces yet. Open or create a repository to add one.
        </p>
      {:else}
        <ul class="wsm__list" role="none">
          {#each workspaces as w (w.path)}
            <li class="wsm__item" class:wsm__item--current={activePath === w.path} class:wsm__item--unreachable={!w.reachable}>
              <button
                class="wsm__row"
                onclick={() => pick(w)}
                disabled={!w.reachable}
                title={w.path}
              >
                <span class="wsm__line">
                  <span class="wsm__name">{w.title ?? basename(w.path)}</span>
                  <span class="wsm__when mono">{fmtAgo(w.last_opened_unix)}</span>
                </span>
                <span class="wsm__path mono">{w.path}</span>
              </button>
              <button class="btn btn--icon wsm__remove" title="forget" onclick={(e) => removeOne(e, w)}>✕</button>
            </li>
          {/each}
        </ul>
        {#if unreachableCount > 0}
          <div class="wsm__foot">
            <span class="meta">{unreachableCount} unreachable</span>
            <button class="btn" onclick={forgetUnreachable}>Forget unreachable</button>
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .wsm {
    position: relative;
    display: inline-flex;
    align-items: baseline;
    gap: var(--s-1);
  }
  .wsm__trigger {
    font: inherit;
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--ink);
    cursor: pointer;
    display: inline-flex;
    align-items: baseline;
    gap: var(--s-2);
    transition: color var(--d-fast) var(--ease-out-quart);
  }
  .wsm__trigger:hover { color: var(--accent); }
  .wsm__title {
    font-size: var(--t-base);
    font-weight: var(--w-sem);
    letter-spacing: var(--track-tight);
  }
  .wsm__title--muted { color: var(--ink-mute); font-weight: var(--w-reg); }
  .wsm__chev {
    font-size: var(--t-xs);
    color: var(--ink-mute);
    transition: transform var(--d-fast) var(--ease-out-quart);
    line-height: 1;
  }
  .wsm__chev--open { transform: translateY(1px) scaleY(-1); }

  .wsm__panel {
    position: absolute;
    top: calc(100% + var(--s-3));
    left: 0;
    z-index: 30;
    min-width: 360px;
    max-width: 520px;
    background: var(--bg-raised);
    border: var(--hairline) solid var(--rule);
    box-shadow: 0 12px 32px -16px oklch(0% 0 0 / 0.18);
    display: flex;
    flex-direction: column;
  }
  .wsm__actions {
    display: flex;
    justify-content: space-between;
    gap: var(--s-4);
    padding: var(--s-2) var(--s-3);
    border-bottom: var(--hairline) solid var(--rule);
  }

  .wsm__list {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 360px;
    overflow: auto;
  }
  .wsm__item {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: stretch;
    border-bottom: var(--hairline) solid var(--rule-soft);
  }
  .wsm__item:last-child { border-bottom: 0; }
  .wsm__item:hover { background: var(--bg-inset); }
  .wsm__item--current { background: var(--bg-inset); }
  .wsm__item--current .wsm__name { color: var(--accent); font-weight: var(--w-sem); }

  .wsm__row {
    font: inherit;
    background: transparent;
    border: 0;
    padding: var(--s-2) var(--s-3);
    text-align: left;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    color: var(--ink);
  }
  .wsm__row:disabled { color: var(--ink-mute); cursor: not-allowed; }

  .wsm__line {
    display: flex;
    justify-content: space-between;
    gap: var(--s-3);
    align-items: baseline;
  }
  .wsm__name {
    font-size: var(--t-sm);
    font-weight: var(--w-med);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .wsm__when {
    font-size: var(--t-xs);
    color: var(--ink-mute);
    flex-shrink: 0;
  }
  .wsm__path {
    font-size: var(--t-xs);
    color: var(--ink-mute);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .wsm__item--unreachable .wsm__name,
  .wsm__item--unreachable .wsm__path { color: var(--ink-mute); text-decoration: line-through; text-decoration-thickness: 0.75px; }

  .wsm__remove {
    align-self: stretch;
    padding: 0 var(--s-3);
    opacity: 0;
    transition: opacity var(--d-fast);
  }
  .wsm__item:hover .wsm__remove,
  .wsm__item--unreachable .wsm__remove { opacity: 1; }

  .wsm__foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--s-2) var(--s-3);
    border-top: var(--hairline) solid var(--rule);
  }
</style>
