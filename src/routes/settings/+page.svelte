<script lang="ts">
    import { open as openDialog } from '@tauri-apps/plugin-dialog';
    import { settings, setVault, clearVault  } from '#lib/stores/settings.svelte.ts';

    async function changeVault() {
        const selected = await openDialog({
            directory: true,
            multiple: false,
            title: 'Change Vault Location'
        });

        if (selected && typeof selected === 'string') {
            setVault(selected);
        }
    }
</script>

<a href="/">Home</a>

<section>
    <h2>Vault</h2>
    <p>Location / Path: {settings.paths.vault}</p>
    <button onclick={changeVault}>Change</button>
    <button onclick={clearVault}>Clear</button>
</section>