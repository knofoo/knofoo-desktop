<script lang="ts">
    import Section from '$lib/components/settings/ui/Section.svelte';
    import Item from '$lib/components/settings/ui/Item.svelte';
    import { vault } from '$lib/stores/vaultStore.svelte';

    let autoSave = $state(true);
    let confirmExit = $state(false);

    const folderName = $derived(vault.path ? vault.path.split(/[\\/]/).filter(Boolean).at(-1) : null);
</script>

<div class="page">
    <h1 class="page-title">General</h1>

    <Section title="Vault">
        <Item label="Location">
            <div class="vault-path">
                {#if vault.path}
                    <span class="full-path" title={vault.path}>{vault.path}</span>
                {:else}
                    <span class="full-path">—</span>
                {/if}
                <button class="pick-btn" onclick={() => vault.pick()}>
                    {vault.path ? 'Change…' : 'Choose folder…'}
                </button>
            </div>
        </Item>
    </Section>

    <Section title="Behavior">
        <Item label="Auto Save">
            <input type="checkbox" bind:checked={autoSave} />
        </Item>
        <Item label="Confirm on Exit">
            <input type="checkbox" bind:checked={confirmExit} />
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

    input[type="checkbox"] {
        accent-color: #e15023;
        width: 16px;
        height: 16px;
        cursor: pointer;
    }

    .vault-path {
        display: flex;
        flex-direction: row;
        align-items: center;
        gap: 10px;
    }

    .folder-name {
        font-size: 1rem;
        color: rgba(226, 232, 240, 0.9);
        font-weight: 500;
    }

    .full-path {
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
</style>
