<script lang="ts">
    import { open as openDialog } from '@tauri-apps/plugin-dialog';
    import { setVault } from '#lib/stores/settings.svelte.ts';

    async function pickExisting() {
        const selected = await openDialog({
            directory: true,
            multiple: false,
            title: 'Open Vault'
        });

        if (selected && typeof selected === 'string') {
            setVault(selected);
        }
    }

    async function create() {
        const selected = await openDialog({
            directory: true,
            multiple: false,
            title: 'Choose Location for New Vault'
        });

        if (selected && typeof selected === 'string') {
            // scaffold vault structure at `selected` here
            setVault(selected);
        }
    }
</script>

<div class="modal-backdrop">
    <div class="modal" role="dialog" aria-modal="true">
        <h2>Open a vault</h2>
        <button onclick={pickExisting}>Open Existing</button>
        <button onclick={create}>Create New</button>
    </div>
</div>