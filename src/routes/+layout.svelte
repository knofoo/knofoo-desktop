<script lang="ts">
    import { onMount } from 'svelte';
    import { settings, hydrateSettings } from '#lib/stores/settings.svelte.ts';
    import VaultModal from '#lib/components/VaultModal.svelte';

    let { children } = $props();
    let ready = $state(false);

    onMount(() => {
        hydrateSettings();
        ready = true;
    });
</script>

{#if ready}
    {#if !settings.paths.vault}
        <VaultModal />
    {:else}
        {@render children()}
    {/if}
{/if}