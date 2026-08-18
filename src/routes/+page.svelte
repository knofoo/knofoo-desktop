<script lang="ts">
    import DockLayout from '$lib/layout/DockLayout.svelte';
    import DockArea from '$lib/layout/DockArea.svelte';
    import DockPanel from '$lib/layout/DockPanel.svelte';
    import ExplorerPanel from '$lib/panels/ExplorerPanel.svelte';
    import GraphPanel from '$lib/panels/GraphPanel.svelte';
    import SettingsPanel from '$lib/panels/SettingsPanel.svelte';
    import { layout } from '$lib/layout/layoutStore.svelte';
    import { logStore } from '$lib/stores/logStore.svelte';

    let graphPanel = $state<GraphPanel | undefined>(undefined);
    let container = $state<HTMLDivElement | undefined>(undefined);

    let activeDivider = $state<'explorer' | 'settings' | null>(null);
    let logVisible = $state(true);
    let logHeight = $state((() => {
        const stored = parseInt(localStorage.getItem('knofoo_log_height') ?? '', 10);
        return Number.isFinite(stored) && stored >= 60 ? stored : 160;
    })());
    let resizingLog = $state(false);

    function startLogResize(e: MouseEvent) {
        e.preventDefault();
        resizingLog = true;
        const startY = e.clientY;
        const startH = logHeight;
        function onMove(ev: MouseEvent) {
            const delta = startY - ev.clientY;
            const next = Math.max(60, Math.min(window.innerHeight - 200, startH + delta));
            logHeight = next;
        }
        function onUp() {
            resizingLog = false;
            localStorage.setItem('knofoo_log_height', String(logHeight));
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
        }
        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
    }

    $effect(() => {
        if (!container) return;
        const obs = new ResizeObserver(entries => {
            layout.containerWidth = entries[0].contentRect.width;
        });
        obs.observe(container);
        return () => obs.disconnect();
    });

    function startDrag(divider: 'explorer' | 'settings', e: MouseEvent) {
        e.preventDefault();
        activeDivider = divider;
        const startX = e.clientX;
        const startWidth = divider === 'explorer' ? layout.explorer.width : layout.settings.width;

        function onMove(e: MouseEvent) {
            const delta = e.clientX - startX;
            if (divider === 'explorer') {
                layout.setExplorerWidth(startWidth + delta);
            } else {
                layout.setSettingsWidth(startWidth - delta);
            }
        }

        function onUp() {
            activeDivider = null;
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
        }

        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
    }

    const noPanels = $derived(!layout.explorer.visible && !layout.graph.visible && !layout.settings.visible);
</script>

<div class="page">
    {#if !logVisible}
        <button class="log-reopen" onclick={() => { logVisible = true; }} title="Show VM Log">⌃ Log</button>
    {/if}
    <div class="workspace" bind:this={container}>
        <DockLayout>
            {#if noPanels}
                <div class="empty-workspace">
                    <span>No panels open — use View to add one</span>
                </div>
            {:else}
                {#if layout.explorer.visible}
                    <div class="side-panel" style="width: {layout.explorer.width}px">
                        <DockArea position="left">
                            <DockPanel id="explorer" title="Explorer">
                                <ExplorerPanel />
                            </DockPanel>
                        </DockArea>
                    </div>
                    {#if layout.graph.visible || layout.settings.visible}
                        <button
                            class="divider"
                            class:active={activeDivider === 'explorer'}
                            aria-label="Resize explorer panel"
                            onmousedown={(e) => startDrag('explorer', e)}
                        ></button>
                    {/if}
                {/if}

                {#if layout.graph.visible}
                    <DockArea position="center">
                        <DockPanel id="graph" title="Graph">
                            <GraphPanel bind:this={graphPanel} />
                        </DockPanel>
                    </DockArea>
                {/if}

                {#if layout.settings.visible}
                    {#if layout.explorer.visible || layout.graph.visible}
                        <button
                            class="divider"
                            class:active={activeDivider === 'settings'}
                            aria-label="Resize settings panel"
                            onmousedown={(e) => startDrag('settings', e)}
                        ></button>
                    {/if}
                    <div class="side-panel" style="width: {layout.settings.width}px">
                        <DockArea position="right">
                            <DockPanel id="settings" title="Settings">
                                <SettingsPanel />
                            </DockPanel>
                        </DockArea>
                    </div>
                {/if}
            {/if}
        </DockLayout>
    </div>
    {#if logVisible}
    <div
        class="log-resizer"
        class:active={resizingLog}
        role="separator"
        aria-orientation="horizontal"
        tabindex="0"
        onmousedown={startLogResize}
    ></div>
    <div class="log-panel" style:height="{logHeight}px">
        <div class="log-header">
            <span>VM Log</span>
            <div class="log-actions">
                <button onclick={() => logStore.clear()} title="Clear">⌫</button>
                <button onclick={() => { logVisible = false; }} title="Close">✕</button>
            </div>
        </div>
        <div class="log-body">
            {#each logStore.entries as e}
                <div class="log-line" class:err={e.kind === 'error'}>
                    <span class="log-t">{e.t}</span> {e.msg}
                </div>
            {/each}
            {#if logStore.entries.length === 0}
                <div class="log-line" style="opacity:0.3">waiting for vm_start...</div>
            {/if}
        </div>
    </div>
    {/if}
</div>

<style>
    .page {
        display: flex;
        flex-direction: column;
        height: 100%;
        width: 100%;
        overflow: hidden;
    }

    .workspace {
        display: flex;
        flex: 1;
        min-height: 0;
        overflow: hidden;
    }

    .side-panel {
        display: flex;
        flex-direction: column;
        flex-shrink: 0;
        min-height: 0;
        overflow: hidden;
    }

    .divider {
        flex: 0 0 4px;
        border: none;
        padding: 0;
        background: rgba(255, 255, 255, 0.06);
        cursor: col-resize;
        transition: background 0.15s;
        z-index: 1;
        outline: none;
    }

    .divider:hover,
    .divider.active {
        background: rgba(225, 80, 35, 0.45);
    }

    .log-panel {
        flex-shrink: 0;
        background: rgba(5,5,10,0.97);
        font-family: monospace;
        font-size: 12px;
        display: flex;
        flex-direction: column;
        min-height: 60px;
    }
    .log-resizer {
        flex-shrink: 0;
        height: 4px;
        background: rgba(255,255,255,0.08);
        cursor: ns-resize;
        transition: background 0.1s;
    }
    .log-resizer:hover, .log-resizer.active {
        background: rgba(225, 80, 35, 0.45);
    }
    .log-header {
        display: flex; justify-content: space-between; align-items: center;
        color: rgba(226,232,240,0.5); font-size: 10px;
        text-transform: uppercase;
        padding: 4px 10px;
        background: rgba(255,255,255,0.03);
        border-bottom: 1px solid rgba(255,255,255,0.06);
        flex-shrink: 0;
    }
    .log-actions { display: flex; gap: 4px; }
    .log-header button {
        background: none; border: none;
        color: rgba(255,255,255,0.4); cursor: pointer;
        font-size: 12px; padding: 0 6px;
    }
    .log-header button:hover { color: #e2e8f0; }
    .log-body {
        flex: 1; overflow-y: auto; padding: 4px 10px;
    }
    .log-line { color: rgba(226,232,240,0.7); padding: 1px 0; }
    .log-line.err { color: #f472b6; }
    .log-t { color: rgba(226,232,240,0.25); margin-right: 6px; }
    .log-reopen {
        position: fixed; bottom: 6px; right: 12px; z-index: 50;
        background: rgba(5,5,10,0.9); color: rgba(226,232,240,0.6);
        border: 1px solid rgba(255,255,255,0.12);
        border-radius: 4px; padding: 3px 10px;
        font-family: monospace; font-size: 11px; cursor: pointer;
    }
    .log-reopen:hover { color: #e2e8f0; border-color: rgba(255,255,255,0.3); }

    .empty-workspace {
        flex: 1;
        display: flex;
        align-items: center;
        justify-content: center;
        color: rgba(226, 232, 240, 0.2);
        font-size: 1rem;
        font-family: monospace;
    }
</style>
