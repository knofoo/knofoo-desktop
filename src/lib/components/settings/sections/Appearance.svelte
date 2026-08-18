<script lang="ts">
    import Section from '$lib/components/settings/ui/Section.svelte';
    import Item from '$lib/components/settings/ui/Item.svelte';
    import { zoom } from '$lib/stores/zoomStore.svelte';

    const themes = ['Dark', 'Light', 'System'];
    let theme = $state('Dark');
</script>

<div class="page">
    <h1 class="page-title">Appearance</h1>

    <Section title="Theme">
        <Item label="Color Theme">
            <select bind:value={theme}>
                {#each themes as t}
                    <option value={t}>{t}</option>
                {/each}
            </select>
        </Item>
    </Section>

    <Section title="Editor">
        <Item label="Zoom">
            <div class="zoom-control">
                <button onclick={() => zoom.decrease()}>−</button>
                <span class="zoom-value">{Math.round(zoom.factor * 100)}%</span>
                <button onclick={() => zoom.increase()}>+</button>
            </div>
        </Item>
    </Section>
</div>

<style>
    .page {
        padding: 2rem;
    }

    .page-title {
        font-size: 1.38rem;
        font-weight: 600;
        color: rgba(226, 232, 240, 0.9);
        margin-bottom: 2rem;
    }

    select {
        background: rgba(255, 255, 255, 0.06);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 4px;
        color: rgba(226, 232, 240, 0.85);
        font-size: 0.92rem;
        padding: 3px 6px;
        outline: none;
    }

    select:focus {
        border-color: rgba(225, 80, 35, 0.5);
    }

    .zoom-control {
        display: flex;
        align-items: center;
        gap: 6px;
    }

    .zoom-control button {
        background: rgba(255, 255, 255, 0.06);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 4px;
        color: rgba(226, 232, 240, 0.85);
        width: 24px;
        height: 24px;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 1rem;
        line-height: 1;
    }

    .zoom-control button:hover {
        background: rgba(255, 255, 255, 0.12);
        border-color: rgba(225, 80, 35, 0.5);
    }

    .zoom-value {
        font-size: 0.92rem;
        color: rgba(226, 232, 240, 0.85);
        min-width: 44px;
        text-align: center;
        font-family: monospace;
    }
</style>
