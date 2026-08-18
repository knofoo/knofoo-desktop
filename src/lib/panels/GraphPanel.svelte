<script lang="ts">
    import { graphStore } from '$lib/stores/graphStore.svelte';
    import TabBar from './TabBar.svelte';
    import GraphCanvas from '$lib/graph/GraphCanvas.svelte';
    import ModulePanel from './ModulePanel.svelte';

    let moduleVisible = $state(false);
    let modulePath    = $state<string | null>(null);

    // Three modes:
    //   'collapsed' — header bar only (~30px)
    //   'split'     — both visible, divider draggable
    //   'expanded'  — module fills the panel, canvas hidden
    let moduleMode = $state<'collapsed' | 'split' | 'expanded'>('split');

    // Module height in pixels (only used when mode === 'split')
    let moduleHeight = $state((() => {
        const stored = parseInt(localStorage.getItem('knofoo_module_height') ?? '', 10);
        return Number.isFinite(stored) && stored >= 60 ? stored : 280;
    })());
    let resizing = $state(false);

    function openModulePanel(path: string) {
        modulePath    = path;
        moduleVisible = true;
        if (moduleMode === 'collapsed') moduleMode = 'split';
    }

    function startResize(e: MouseEvent) {
        if (moduleMode !== 'split') return;
        e.preventDefault();
        resizing = true;
        const startY = e.clientY;
        const startH = moduleHeight;
        function onMove(ev: MouseEvent) {
            const delta = startY - ev.clientY;
            // Allow up to 80% of viewport height; let CSS min-height clamp the lower bound.
            const next = Math.max(60, Math.min(window.innerHeight * 0.8, startH + delta));
            moduleHeight = next;
        }
        function onUp() {
            resizing = false;
            localStorage.setItem('knofoo_module_height', String(moduleHeight));
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
        }
        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
    }

    function cycleMode(next: 'collapsed' | 'split' | 'expanded') {
        moduleMode = next;
    }

    function close() {
        moduleVisible = false;
        modulePath = null;
        moduleMode = 'split';
    }
</script>

<div class="graph-panel">
    <TabBar />

    <div class="canvas-area" class:module-expanded={moduleVisible && moduleMode === 'expanded'}>
        {#each graphStore.tabs as tab (tab.id)}
            <div class="canvas-slot" class:hidden={tab.id !== graphStore.active}>
                <GraphCanvas
                    {tab}
                    onModuleNodeDblClick={openModulePanel}
                />
            </div>
        {/each}
        {#if graphStore.tabs.length === 0}
            <div class="empty-state">
                <span class="empty-icon">◈</span>
                <span class="empty-text">Open a graph or module from the sidebar</span>
            </div>
        {/if}
    </div>

    {#if moduleVisible && moduleMode === 'split'}
        <button
            class="module-resizer"
            class:active={resizing}
            aria-label="Resize module panel"
            onmousedown={startResize}
        ></button>
    {/if}

    {#if moduleVisible}
        <div
            class="module-wrap"
            style:height={
                moduleMode === 'expanded' ? '100%' :
                moduleMode === 'split'    ? `${moduleHeight}px` :
                                             '30px'
            }
        >
            <ModulePanel
                visible={moduleVisible}
                expanded={moduleMode !== 'collapsed'}
                mode={moduleMode}
                path={modulePath}
                onmodechange={cycleMode}
                onclose={close}
                onpathchange={(oldPath, newPath) => {
                    modulePath = newPath;
                    window.dispatchEvent(new CustomEvent('knofoo:module-renamed', {
                        detail: { oldPath, newPath }
                    }));
                }}
            />
        </div>
    {/if}
</div>

<style>
    .graph-panel {
        display: flex;
        flex-direction: column;
        flex: 1;
        min-height: 0;
        overflow: hidden;
    }

    .canvas-area {
        flex: 1;
        min-height: 0;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        position: relative;
    }
    .canvas-area.module-expanded {
        display: none;
    }

    .canvas-slot {
        position: absolute;
        inset: 0;
        display: flex;
        flex-direction: column;
    }

    .canvas-slot.hidden {
        visibility: hidden;
        pointer-events: none;
    }

    .empty-state {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 12px;
        background: #0d0d12;
    }

    .empty-icon { font-size: 2rem; color: rgba(225,80,35,0.2); }
    .empty-text { font-size: 0.88rem; font-family: monospace; color: rgba(226,232,240,0.15); }

    .module-resizer {
        flex-shrink: 0;
        height: 4px;
        background: rgba(255,255,255,0.06);
        border: none;
        cursor: ns-resize;
        padding: 0;
        transition: background 0.1s;
    }
    .module-resizer:hover, .module-resizer.active {
        background: rgba(225,80,35,0.45);
    }

    .module-wrap {
        flex-shrink: 0;
        display: flex;
        flex-direction: column;
        min-height: 30px;
        overflow: hidden;
    }
</style>
