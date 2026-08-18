<script lang="ts">
    import { getContext } from 'svelte';
    import type { Snippet } from 'svelte';
    import { DOCK_CONTEXT, type DockContext } from './DockLayout.svelte';

    let {
        id,
        title,
        children,
    }: {
        id: string;
        title: string;
        children: Snippet;
    } = $props();

    const dock = getContext<DockContext>(DOCK_CONTEXT);

    let isExpanded = $derived(dock.isExpanded(id));
</script>

<div class="dock-panel" class:expanded={isExpanded}>
    <div class="dock-panel-header">
        <span class="dock-panel-title">{title}</span>
        <button
            class="dock-panel-toggle"
            onclick={() => dock.toggle(id)}
            title={isExpanded ? 'Restore' : 'Maximize'}
        >
            {#if isExpanded}⊡{:else}⊞{/if}
        </button>
    </div>
    <div class="dock-panel-content">
        {@render children()}
    </div>
</div>

<style>
    .dock-panel {
        display: flex;
        flex-direction: column;
        flex: 1;
        min-height: 0;
        min-width: 0;
        overflow: hidden;
        background: #0b0b14;
    }

    .dock-panel.expanded {
        position: fixed;
        top: 32px;
        left: 0;
        right: 0;
        bottom: 0;
        z-index: 50;
        background: #0b0b14;
    }

    .dock-panel-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        height: 32px;
        min-height: 32px;
        padding: 0 10px 0 12px;
        background: #0f0f17;
        border-bottom: 1px solid rgba(255, 255, 255, 0.07);
        flex-shrink: 0;
        user-select: none;
    }

    .dock-panel-title {
        font-size: 0.85rem;
        font-weight: 600;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        color: rgba(226, 232, 240, 0.4);
        font-family: monospace;
    }

    .dock-panel-toggle {
        background: none;
        border: 1px solid rgba(255, 255, 255, 0.1);
        color: rgba(226, 232, 240, 0.4);
        width: 22px;
        height: 22px;
        border-radius: 4px;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 1rem;
        padding: 0;
        line-height: 1;
        transition: color 0.15s, border-color 0.15s, background 0.15s;
    }

    .dock-panel-toggle:hover {
        background: rgba(255, 255, 255, 0.08);
        border-color: rgba(255, 255, 255, 0.2);
        color: rgba(226, 232, 240, 0.9);
    }

    .dock-panel-content {
        display: flex;
        flex: 1;
        min-height: 0;
        min-width: 0;
        overflow: hidden;
    }
</style>
