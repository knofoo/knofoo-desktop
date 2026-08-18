<script>
    import Sidebar from '$lib/components/settings/Sidebar.svelte';

    let { children } = $props();
</script>

<div class="settings-root">
    <Sidebar />

    <main class="settings-content">
        {@render children()}
    </main>
</div>

<style>
    .settings-root {
        display: flex;
        flex: 1;
        min-height: 0;
        width: 100%;
        background: #0f0f17;
        overflow: hidden;
    }

    .settings-content {
        flex: 1;
        overflow-y: auto;
        min-width: 0;
    }
</style>