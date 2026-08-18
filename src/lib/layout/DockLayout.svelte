<script module lang="ts">
    export const DOCK_CONTEXT = Symbol('dock');

    export interface DockContext {
        isExpanded: (id: string) => boolean;
        toggle: (id: string) => void;
    }
</script>

<script lang="ts">
    import { setContext } from 'svelte';
    import type { Snippet } from 'svelte';

    let { children }: { children: Snippet } = $props();

    let expandedPanelId = $state<string | null>(null);

    setContext<DockContext>(DOCK_CONTEXT, {
        isExpanded: (id: string) => expandedPanelId === id,
        toggle: (id: string) => { expandedPanelId = expandedPanelId === id ? null : id; },
    });
</script>

<div class="dock-layout">
    {@render children()}
</div>

<style>
    .dock-layout {
        position: relative;
        display: flex;
        flex-direction: row;
        flex: 1;
        min-height: 0;
        min-width: 0;
        overflow: hidden;
        background: #0b0b14;
    }
</style>
