<script lang="ts">
    import '$lib/styles/app.css';
    import TopBar from '$lib/components/TopBar.svelte';
    import VaultSetup from '$lib/components/VaultSetup.svelte';
    import { layout } from '$lib/layout/layoutStore.svelte';
    import { vault } from '$lib/stores/vaultStore.svelte';
    import { zoom } from '$lib/stores/zoomStore.svelte';
    import { page } from '$app/state';
    import { getCurrentWebview } from '@tauri-apps/api/webview';

    let { children } = $props();

    const isWorkspace = $derived(page.url.pathname === '/');

    $effect(() => {
        getCurrentWebview().setZoom(zoom.factor);
    });

    $effect(() => {
        function onKeydown(e: KeyboardEvent) {
            if (!e.ctrlKey) return;
            if (e.key === '=' || e.key === '+') {
                e.preventDefault();
                zoom.increase();
            } else if (e.key === '-') {
                e.preventDefault();
                zoom.decrease();
            } else if (e.key === '0') {
                e.preventDefault();
                zoom.reset();
            }
        }
        window.addEventListener('keydown', onKeydown);
        return () => window.removeEventListener('keydown', onKeydown);
    });
</script>

<div class="layout">
    {#if vault.isConfigured}
        <TopBar
            showExplorer={layout.explorer.visible}
            showGraph={layout.graph.visible}
            showSettings={layout.settings.visible}
            ontoggleExplorer={() => layout.toggleExplorer()}
            ontoggleGraph={() => layout.toggleGraph()}
            ontoggleSettings={() => layout.toggleSettings()}
            workspaceControls={isWorkspace}
        />
        {@render children()}
    {:else}
        <VaultSetup />
    {/if}
</div>

<style>
    .layout {
        display: flex;
        flex-direction: column;
        height: 100vh;
        width: 100vw;
        overflow: hidden;
    }
</style>
