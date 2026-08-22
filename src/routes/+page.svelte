<script lang="ts">
    import { settings } from '#lib/stores/settings.svelte.ts';
    import { VaultDirectory } from '#lib/vault/entries.svelte.ts';

    const dir = new VaultDirectory();

    $effect(() => {
        dir.load(settings.paths.vault);
    });
</script>

<a href="/settings">Settings</a>

<h1>{settings.paths.vault}</h1>

{#if dir.loading}
    <p>Loading…</p>
{:else if dir.error}
    <p class="error">{dir.error}</p>
{:else if dir.entries.length === 0}
    <p>No files found.</p>
{:else}
    <ul>
        {#each dir.entries as entry}
            <li>{entry.isDirectory ? '📁' : '📄'} {entry.name}</li>
        {/each}
    </ul>
{/if}