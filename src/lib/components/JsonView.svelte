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

<div class="col" style="height: 100%;">
  <div class="row" style="justify-content: space-between;">
    <span class="muted">Raw geocontext.json — edits apply on click</span>
    <button class="primary" onclick={apply}>apply</button>
  </div>
  {#if err}<span class="error">JSON: {err}</span>{/if}
  <textarea bind:value={text} style="flex: 1; min-height: 0;"></textarea>
</div>
