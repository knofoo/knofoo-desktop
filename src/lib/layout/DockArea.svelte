<script lang="ts">
    import type { Snippet } from 'svelte';

    let {
        position,
        children,
    }: {
        position: 'left' | 'center' | 'right' | 'bottom';
        children: Snippet;
    } = $props();
</script>

<div class="dock-area dock-area--{position}">
    {@render children()}
</div>

<style>
    .dock-area {
        display: flex;
        flex-direction: column;
        flex: 1;
        min-height: 0;
        min-width: 0;
        overflow: hidden;
    }

    .dock-area--left {
        border-right: 1px solid rgba(255, 255, 255, 0.07);
    }

    .dock-area--right {
        border-left: 1px solid rgba(255, 255, 255, 0.07);
    }

    .dock-area--bottom {
        border-top: 1px solid rgba(255, 255, 255, 0.07);
    }
</style>
