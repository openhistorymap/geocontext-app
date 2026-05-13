<script lang="ts">
  import type { GeoContext } from '$lib/types';

  let { model = $bindable() }: { model: GeoContext } = $props();
  let text = $state(JSON.stringify(model, null, 2));
  let err = $state<string | null>(null);

  $effect(() => {
    text = JSON.stringify(model, null, 2);
    err = null;
  });

  function apply() {
    try {
      model = JSON.parse(text) as GeoContext;
      err = null;
    } catch (e) {
      err = (e as Error).message;
    }
  }
</script>

<div class="jv">
  <div class="section__head" style="padding-block: 0 var(--s-3);">
    <div class="col" style="gap: 2px;">
      <span class="section__title">Source</span>
      <span class="section__hint">Edits land on Apply. Use this for fields the form doesn't expose.</span>
    </div>
    <button class="btn btn--primary" onclick={apply}>Apply</button>
  </div>
  {#if err}<span class="error" style="margin-bottom: var(--s-2);">JSON: {err}</span>{/if}
  <textarea class="jv__editor" bind:value={text} spellcheck="false"></textarea>
</div>

<style>
  .jv {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .jv__editor {
    flex: 1;
    min-height: 0;
    font-family: var(--font-mono);
    font-size: var(--t-sm);
    line-height: 1.6;
    background: var(--bg-raised);
    color: var(--ink);
    border: var(--hairline) solid var(--rule);
    padding: var(--s-3);
    resize: none;
    width: 100%;
  }
  .jv__editor:focus { outline: none; border-color: var(--accent); }
</style>
