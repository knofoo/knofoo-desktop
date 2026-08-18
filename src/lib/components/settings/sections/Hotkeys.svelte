<script lang="ts">
    import Section from '$lib/components/settings/ui/Section.svelte';
    import Item from '$lib/components/settings/ui/Item.svelte';

    const bindings = [
        { label: 'Toggle Explorer', key: 'Ctrl+Shift+E' },
        { label: 'Toggle Graph',    key: 'Ctrl+Shift+G' },
        { label: 'Open Settings',   key: 'Ctrl+,' },
        { label: 'Go to Workspace', key: 'Escape' },
        { label: 'Zoom In',         key: 'Ctrl+=' },
        { label: 'Zoom Out',        key: 'Ctrl+-' },
        { label: 'Reset Zoom',      key: 'Ctrl+0' },
    ];
</script>

<div class="page">
    <h1 class="page-title">Hotkeys</h1>

    <Section title="Navigation">
        {#each bindings as binding}
            <Item label={binding.label}>
                <kbd class="key">{binding.key}</kbd>
            </Item>
        {/each}
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

    .key {
        font-family: monospace;
        font-size: 0.85rem;
        background: rgba(255, 255, 255, 0.08);
        border: 1px solid rgba(255, 255, 255, 0.12);
        border-radius: 4px;
        padding: 2px 6px;
        color: rgba(226, 232, 240, 0.7);
    }
</style>
