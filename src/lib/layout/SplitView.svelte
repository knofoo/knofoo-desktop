<script lang="ts">
    import type { Snippet } from 'svelte';

    let {
        direction = 'horizontal',
        initialRatio = 0.5,
        minRatio = 0.1,
        maxRatio = 0.9,
        start,
        end,
    }: {
        direction?: 'horizontal' | 'vertical';
        initialRatio?: number;
        minRatio?: number;
        maxRatio?: number;
        start: Snippet;
        end: Snippet;
    } = $props();

    // svelte-ignore state_referenced_locally
    let ratio = $state(initialRatio);
    let dragging = $state(false);
    let container: HTMLDivElement;

    function onDividerMouseDown(e: MouseEvent) {
        e.preventDefault();
        dragging = true;
        window.addEventListener('mousemove', onMouseMove);
        window.addEventListener('mouseup', onMouseUp);
    }

    function onMouseMove(e: MouseEvent) {
        if (!dragging || !container) return;
        const rect = container.getBoundingClientRect();
        const raw = direction === 'horizontal'
            ? (e.clientX - rect.left) / rect.width
            : (e.clientY - rect.top) / rect.height;
        ratio = Math.min(maxRatio, Math.max(minRatio, raw));
    }

    function onMouseUp() {
        dragging = false;
        window.removeEventListener('mousemove', onMouseMove);
        window.removeEventListener('mouseup', onMouseUp);
    }

    let startSize = $derived(
        direction === 'horizontal'
            ? `width: ${ratio * 100}%`
            : `height: ${ratio * 100}%`
    );
</script>

<div
    class="split-view split-view--{direction}"
    class:dragging
    bind:this={container}
>
    <div class="split-pane split-pane--start" style={startSize}>
        {@render start()}
    </div>
    <button
        class="split-divider"
        aria-label="Resize panels"
        onmousedown={onDividerMouseDown}
    ></button>
    <div class="split-pane split-pane--end">
        {@render end()}
    </div>
</div>

<style>
    .split-view {
        display: flex;
        flex: 1;
        min-height: 0;
        min-width: 0;
        overflow: hidden;
    }

    .split-view--horizontal {
        flex-direction: row;
    }

    .split-view--vertical {
        flex-direction: column;
    }

    .split-pane {
        display: flex;
        flex-direction: column;
        min-height: 0;
        min-width: 0;
        overflow: hidden;
        flex-shrink: 0;
    }

    .split-pane--end {
        flex: 1;
        flex-shrink: 1;
    }

    .split-divider {
        flex: 0 0 4px;
        background: rgba(255, 255, 255, 0.06);
        border: none;
        padding: 0;
        cursor: col-resize;
        transition: background 0.15s;
        z-index: 1;
        flex-shrink: 0;
    }

    .split-view--vertical .split-divider {
        cursor: row-resize;
    }

    .split-divider:hover,
    .dragging .split-divider {
        background: rgba(225, 80, 35, 0.45);
    }
</style>
