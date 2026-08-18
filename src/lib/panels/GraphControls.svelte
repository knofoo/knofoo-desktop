<script lang="ts">
  import { Panel, useSvelteFlow } from '@xyflow/svelte';

  let { showMinimap = $bindable(true) } = $props();

  const { fitView, setCenter } = useSvelteFlow();

  export function fit() { fitView({ padding: 0.2, duration: 400 }); }
  export function recenter() { setCenter(0, 0, { zoom: 1, duration: 400 }); }
</script>

<Panel position="top-right">
  <div class="toolbar">
    <button onclick={() => fitView({ padding: 0.2, duration: 400 })} title="Fit view">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M3 7V3h4M21 7V3h-4M3 17v4h4M21 17v4h-4"/>
      </svg>
      Fit
    </button>
    <button onclick={() => setCenter(0, 0, { zoom: 1, duration: 400 })} title="Re-center">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3"/><circle cx="12" cy="12" r="9"/>
      </svg>
      Center
    </button>
    <button onclick={() => showMinimap = !showMinimap} title="Toggle minimap">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect x="3" y="3" width="18" height="18" rx="2"/><rect x="7" y="13" width="4" height="4"/>
      </svg>
      Map
    </button>
  </div>
</Panel>

<style>
  .toolbar {
    display: flex;
    gap: 4px;
    padding: 4px;
  }

  .toolbar button {
    display: flex;
    align-items: center;
    gap: 5px;
    background: #111;
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 6px;
    color: rgba(226,232,240,0.7);
    font-size: 12px;
    padding: 5px 10px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .toolbar button:hover {
    background: #1f1f1f;
    color: #e2e8f0;
  }
</style>