<script lang="ts">
    import { open } from '@tauri-apps/plugin-dialog';
    import { invoke } from '@tauri-apps/api/core';
    import { vault } from '$lib/stores/vaultStore.svelte';

    let error = $state('');
    let loading = $state(false);

    async function pickFolder() {
        error = '';
        loading = true;
        try {
            const selected = await open({ directory: true, multiple: false });
            if (!selected) return;
            await invoke('init_vault', { path: selected });
            vault.set(selected);
        } catch (e) {
            error = String(e);
        } finally {
            loading = false;
        }
    }
</script>

<div class="vault-setup">
    <div class="card">
        <div class="logo">K</div>
        <p class="tagline">Where should we keep your knowledge?</p>
        <button class="btn-primary" onclick={pickFolder} disabled={loading}>
            {loading ? 'Setting up…' : 'Choose folder'}
        </button>
        {#if error}
            <span class="error">{error}</span>
        {/if}
    </div>
</div>

<style>
    .vault-setup {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        width: 100%;
        background: #0f0f17;
    }

    .card {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 20px;
        width: 340px;
        padding: 48px 36px;
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.07);
        border-radius: 10px;
    }

    .logo {
        width: 52px;
        height: 52px;
        background: rgb(225, 80, 35);
        border-radius: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
        font-family: monospace;
        font-size: 1.85rem;
        font-weight: 700;
        color: #fff;
    }

    .tagline {
        margin: 0;
        font-size: 1rem;
        color: rgba(226, 232, 240, 0.45);
        text-align: center;
    }

    .btn-primary {
        width: 100%;
        padding: 10px;
        background: rgb(225, 80, 35);
        border: none;
        border-radius: 6px;
        color: #fff;
        font-size: 1rem;
        font-weight: 600;
        cursor: pointer;
        transition: opacity 0.15s;
    }

    .btn-primary:hover:not(:disabled) {
        opacity: 0.85;
    }

    .btn-primary:disabled {
        opacity: 0.4;
        cursor: default;
    }

    .error {
        font-size: 0.85rem;
        color: rgba(225, 80, 35, 0.9);
        font-family: monospace;
        text-align: center;
    }
</style>
