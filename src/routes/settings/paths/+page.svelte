<script lang="ts">
    import Section from '$lib/components/settings/ui/Section.svelte';
    import Item from '$lib/components/settings/ui/Item.svelte';
    import { vault, type KnofooConfig } from '$lib/stores/vaultStore.svelte';
    import Dropdown from '$lib/components/Dropdown.svelte';

    let saving = $state(false);
    let saved  = $state(false);

    type AssetResolution = 'vault' | 'next-to-module' | 'custom';

    let graphs          = $state(vault.config.paths.graphs);
    let modules         = $state(vault.config.paths.modules);
    let notes           = $state(vault.config.paths.notes);
    let assets          = $state(vault.config.paths.assets);
    let assetResolution = $state<AssetResolution>((vault.config as any).assetResolution ?? 'vault');

    $effect(() => {
        graphs          = vault.config.paths.graphs;
        modules         = vault.config.paths.modules;
        notes           = vault.config.paths.notes;
        assets          = vault.config.paths.assets;
        assetResolution = (vault.config as any).assetResolution ?? 'vault';
    });

    async function save() {
        saving = true;
        const updated: KnofooConfig = {
            ...vault.config,
            paths: { graphs, modules, notes, assets },
            assetResolution,
        } as any;
        await vault.saveConfig(updated);
        saving = false;
        saved  = true;
        setTimeout(() => { saved = false; }, 2000);
    }

    function reset() {
        graphs          = '.knofoo/graphs';
        modules         = '.knofoo/modules';
        notes           = '.knofoo/notes';
        assets          = '.knofoo/assets';
        assetResolution = 'vault';
    }
</script>

<div class="page">
    <h1 class="page-title">Paths</h1>

    <Section title="Vault">
        <Item label="Location">
            <div class="vault-path">
                {#if vault.path}
                    <span class="vault-full" title={vault.path}>{vault.path}</span>
                {:else}
                    <span class="vault-full">—</span>
                {/if}
                <button class="pick-btn" onclick={() => vault.pick()}>
                    {vault.path ? 'Change…' : 'Choose folder…'}
                </button>
            </div>
        </Item>
    </Section>

    <Section title="Storage">
        <Item label="Graphs">
            <input class="path-input" bind:value={graphs} spellcheck="false" />
        </Item>
        <Item label="Modules">
            <input class="path-input" bind:value={modules} spellcheck="false" />
        </Item>
        <Item label="Notes">
            <input class="path-input" bind:value={notes} spellcheck="false" />
        </Item>
        <Item label="Assets">
            <input class="path-input" bind:value={assets} spellcheck="false" />
        </Item>
    </Section>

    <Section title="Asset Resolution">
        <Item label="Resolve assets from">
            <Dropdown
                value={assetResolution}
                onchange={(v) => assetResolution = v as AssetResolution}
                options={[
                    { value: 'vault',          label: 'Vault root' },
                    { value: 'next-to-module', label: 'Next to module file' },
                    { value: 'custom',         label: 'Assets folder (above)' },
                ]}
            />
        </Item>
    </Section>

    <div class="actions">
        <button class="btn-reset" onclick={reset}>Reset to defaults</button>
        <button class="btn-save" onclick={save} disabled={saving}>
            {#if saving}Saving…{:else if saved}Saved{:else}Save{/if}
        </button>
    </div>

    <p class="hint">All paths are relative to the vault root. Changing them does not move existing files.</p>
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

    .vault-path {
        display: flex;
        flex-direction: row;
        align-items: center;
        gap: 10px;
    }

    .vault-name {
        font-size: 1rem;
        color: rgba(226, 232, 240, 0.9);
        font-weight: 500;
    }

    .vault-full {
        font-size: 0.85rem;
        color: rgba(226, 232, 240, 0.35);
        font-family: monospace;
        max-width: 220px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        direction: rtl;
        text-align: right;
    }
    .pick-btn {
        padding: 4px 12px;
        background: rgba(225, 80, 35, 0.15);
        border: 1px solid rgba(225, 80, 35, 0.4);
        border-radius: 4px;
        color: rgba(226, 232, 240, 0.9);
        font-size: 0.85rem;
        cursor: pointer;
        font-family: monospace;
    }
    .pick-btn:hover {
        background: rgba(225, 80, 35, 0.3);
    }

    .path-input {
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 5px;
        color: rgba(226, 232, 240, 0.85);
        font-family: monospace;
        font-size: 0.9rem;
        padding: 4px 8px;
        width: 220px;
        outline: none;
        transition: border-color 0.15s;
    }

    .path-input:focus {
        border-color: rgba(225, 80, 35, 0.6);
    }

    .actions {
        display: flex;
        gap: 8px;
        margin-top: 2rem;
        justify-content: flex-end;
    }

    .btn-reset {
        padding: 6px 16px;
        background: none;
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 5px;
        color: rgba(226, 232, 240, 0.45);
        font-size: 0.9rem;
        cursor: pointer;
        transition: all 0.15s;
    }

    .btn-reset:hover {
        background: rgba(255, 255, 255, 0.05);
        color: rgba(226, 232, 240, 0.7);
    }

    .btn-save {
        padding: 6px 20px;
        background: rgba(225, 80, 35, 0.2);
        border: 1px solid rgba(225, 80, 35, 0.5);
        border-radius: 5px;
        color: rgba(226, 232, 240, 0.9);
        font-size: 0.9rem;
        cursor: pointer;
        transition: all 0.15s;
    }

    .btn-save:hover:not(:disabled) {
        background: rgba(225, 80, 35, 0.35);
    }

    .btn-save:disabled {
        opacity: 0.6;
        cursor: default;
    }

    .select-input {
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 5px;
        color: rgba(226, 232, 240, 0.85);
        font-size: 0.9rem;
        padding: 4px 8px;
        width: 220px;
        outline: none;
        cursor: pointer;
        transition: border-color 0.15s;
    }

    .select-input:focus {
        border-color: rgba(225, 80, 35, 0.6);
    }

    .hint {
        margin-top: 1rem;
        font-size: 0.82rem;
        color: rgba(226, 232, 240, 0.25);
        font-family: monospace;
    }
</style>
