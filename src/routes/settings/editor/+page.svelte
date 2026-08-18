<script lang="ts">
    import Section from '$lib/components/settings/ui/Section.svelte';
    import Item from '$lib/components/settings/ui/Item.svelte';
    import { vault } from '$lib/stores/vaultStore.svelte';

    let saving = $state(false);
    let saved  = $state(false);

    let snapToGrid = $state(vault.config.editor?.snapToGrid ?? false);
    let gridSize   = $state(vault.config.editor?.gridSize   ?? 10);

    $effect(() => {
        snapToGrid = vault.config.editor?.snapToGrid ?? false;
        gridSize   = vault.config.editor?.gridSize   ?? 10;
    });

    async function save() {
        saving = true;
        await vault.saveConfig({
            ...vault.config,
            editor: { snapToGrid, gridSize },
        });
        saving = false;
        saved  = true;
        setTimeout(() => { saved = false; }, 2000);
    }
</script>

<div class="page">
    <h1 class="page-title">Editor</h1>

    <Section title="Canvas">
        <Item label="Snap to Grid">
            <label class="toggle">
                <input type="checkbox" bind:checked={snapToGrid} onchange={save} />
                <span class="toggle-label">{snapToGrid ? 'On' : 'Off'}</span>
            </label>
        </Item>
        <Item label="Grid Size">
            <div class="grid-size-row">
                <input
                    class="number-input"
                    type="number"
                    min="5" max="50" step="5"
                    bind:value={gridSize}
                    disabled={!snapToGrid}
                />
                <span class="unit">px</span>
            </div>
        </Item>
    </Section>

    <div class="actions">
        <button class="btn-save" onclick={save} disabled={saving}>
            {#if saving}Saving…{:else if saved}Saved{:else}Save{/if}
        </button>
    </div>
</div>

<style>
    .page { padding: 2rem; }

    .page-title {
        font-size: 1.38rem;
        font-weight: 600;
        color: rgba(226,232,240,0.9);
        margin-bottom: 2rem;
    }

    .toggle {
        display: flex;
        align-items: center;
        gap: 8px;
        cursor: pointer;
    }

    input[type="checkbox"] {
        accent-color: #e15023;
        width: 16px;
        height: 16px;
        cursor: pointer;
    }

    .toggle-label {
        font-size: 0.9rem;
        color: rgba(226,232,240,0.6);
    }

    .grid-size-row {
        display: flex;
        align-items: center;
        gap: 6px;
    }

    .number-input {
        width: 70px;
        background: rgba(255,255,255,0.05);
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 5px;
        color: rgba(226,232,240,0.85);
        font-size: 0.9rem;
        padding: 4px 8px;
        outline: none;
    }

    .number-input:disabled { opacity: 0.4; }
    .number-input:focus    { border-color: rgba(225,80,35,0.6); }

    .unit {
        font-size: 0.82rem;
        color: rgba(226,232,240,0.35);
        font-family: monospace;
    }

    .actions { display: flex; justify-content: flex-end; margin-top: 2rem; }

    .btn-save {
        padding: 6px 20px;
        background: rgba(225,80,35,0.2);
        border: 1px solid rgba(225,80,35,0.5);
        border-radius: 5px;
        color: rgba(226,232,240,0.9);
        font-size: 0.9rem;
        cursor: pointer;
        transition: all 0.15s;
    }
    .btn-save:hover:not(:disabled) { background: rgba(225,80,35,0.35); }
    .btn-save:disabled { opacity: 0.6; cursor: default; }
</style>
