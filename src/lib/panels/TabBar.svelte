<script lang="ts">
    import { graphStore, type Tab } from '$lib/stores/graphStore.svelte';

    function kindIcon(tab: Tab) {
        return tab.kind === 'module' ? '⊞' : '◇';
    }

    let confirmId  = $state<string | null>(null);
    let confirmTab = $derived(graphStore.tabs.find(t => t.id === confirmId) ?? null);

    function tryClose(id: string) {
        const tab = graphStore.tabs.find(t => t.id === id);
        if (tab?.dirty) {
            confirmId = id;
        } else {
            graphStore.close(id);
        }
    }

    async function doSaveClose() {
        if (!confirmId) return;
        await graphStore.save(confirmId);
        graphStore.close(confirmId);
        confirmId = null;
    }

    function doDiscard() {
        if (!confirmId) return;
        graphStore.close(confirmId);
        confirmId = null;
    }

    function onMiddleClick(e: MouseEvent, id: string) {
        if (e.button === 1) { e.preventDefault(); tryClose(id); }
    }
</script>

<div class="tab-bar">
    {#each graphStore.tabs as tab (tab.id)}
        <div
            class="tab"
            class:active={tab.id === graphStore.active}
            class:module={tab.kind === 'module'}
            role="tab"
            tabindex="0"
            aria-selected={tab.id === graphStore.active}
            onclick={() => graphStore.setActive(tab.id)}
            onkeydown={(e) => e.key === 'Enter' && graphStore.setActive(tab.id)}
            onmousedown={(e) => onMiddleClick(e, tab.id)}
            title={tab.path}
        >
            <span class="tab-icon">{kindIcon(tab)}</span>
            <span class="tab-name">{tab.name}</span>
            {#if tab.dirty}
                <span class="tab-dirty" title="Unsaved">●</span>
            {/if}
            <button
                class="tab-close"
                onclick={(e) => { e.stopPropagation(); tryClose(tab.id); }}
                aria-label="Close {tab.name}"
            >×</button>
        </div>
    {/each}

    {#if graphStore.tabs.length === 0}
        <span class="tab-empty">Open a graph or module from the sidebar</span>
    {/if}
</div>

{#if confirmTab}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal-backdrop" onmousedown={() => { confirmId = null; }}>
        <div class="modal" onmousedown={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
            <div class="modal-title">Unsaved changes</div>
            <div class="modal-body">
                <strong>{confirmTab.name}</strong> has unsaved changes.
            </div>
            <div class="modal-actions">
                <button class="btn-save" onclick={doSaveClose}>Save</button>
                <button class="btn-discard" onclick={doDiscard}>Discard</button>
                <button class="btn-cancel" onclick={() => { confirmId = null; }}>Cancel</button>
            </div>
        </div>
    </div>
{/if}

<style>
    .tab-bar {
        display: flex;
        align-items: stretch;
        background: #0a0a12;
        border-bottom: 1px solid rgba(255,255,255,0.07);
        overflow-x: auto;
        overflow-y: hidden;
        flex-shrink: 0;
        height: 34px;
        scrollbar-width: none;
    }

    .tab-bar::-webkit-scrollbar { display: none; }

    .tab {
        display: flex;
        align-items: center;
        gap: 5px;
        padding: 0 10px 0 10px;
        min-width: 100px;
        max-width: 200px;
        height: 100%;
        background: none;
        border: none;
        border-right: 1px solid rgba(255,255,255,0.05);
        color: rgba(226,232,240,0.45);
        font-size: 0.85rem;
        cursor: pointer;
        white-space: nowrap;
        position: relative;
        flex-shrink: 0;
        transition: background 0.1s, color 0.1s;
    }

    .tab:hover { background: rgba(255,255,255,0.04); color: rgba(226,232,240,0.75); }
    .tab.active { background: #111318; color: rgba(226,232,240,0.95); border-bottom: 2px solid #e15023; }
    .tab.active.module { border-bottom-color: #818cf8; }

    .tab-icon { font-size: 0.72rem; color: rgba(225,80,35,0.6); flex-shrink: 0; }
    .tab.active.module .tab-icon { color: rgba(129,140,248,0.8); }

    .tab-name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; text-align: left; }

    .tab-dirty { font-size: 0.5rem; color: #fbbf24; flex-shrink: 0; }

    .tab-close {
        display: flex; align-items: center; justify-content: center;
        width: 16px; height: 16px;
        background: none; border: none; border-radius: 3px;
        color: rgba(226,232,240,0.3); font-size: 0.9rem; cursor: pointer;
        padding: 0; flex-shrink: 0; line-height: 1;
        opacity: 0; transition: opacity 0.1s, background 0.1s, color 0.1s;
    }
    .tab:hover .tab-close, .tab.active .tab-close { opacity: 1; }
    .tab-close:hover { background: rgba(239,68,68,0.2); color: rgb(239,68,68); }

    .tab-empty {
        display: flex; align-items: center; padding: 0 14px;
        font-size: 0.78rem; font-family: monospace; color: rgba(226,232,240,0.15);
    }

    /* Modal */
    .modal-backdrop {
        position: fixed; inset: 0; z-index: 1000;
        background: rgba(0,0,0,0.5);
        display: flex; align-items: center; justify-content: center;
    }

    .modal {
        background: #1a1a2e;
        border: 1px solid rgba(255,255,255,0.12);
        border-radius: 10px;
        padding: 20px 24px;
        min-width: 300px;
        box-shadow: 0 16px 48px rgba(0,0,0,0.7);
        display: flex; flex-direction: column; gap: 14px;
    }

    .modal-title {
        font-size: 0.95rem; font-weight: 600;
        color: rgba(226,232,240,0.9);
    }

    .modal-body {
        font-size: 0.88rem; color: rgba(226,232,240,0.6); line-height: 1.5;
    }
    .modal-body strong { color: rgba(226,232,240,0.9); }

    .modal-actions { display: flex; gap: 8px; justify-content: flex-end; }

    .modal-actions button {
        padding: 6px 16px; border-radius: 5px; font-size: 0.88rem;
        cursor: pointer; border: 1px solid transparent; transition: all 0.1s;
    }

    .btn-save {
        background: rgba(225,80,35,0.85); border-color: rgba(225,80,35,0.6);
        color: #fff;
    }
    .btn-save:hover { background: rgba(225,80,35,1); }

    .btn-discard {
        background: rgba(239,68,68,0.15); border-color: rgba(239,68,68,0.3);
        color: rgba(239,68,68,0.9);
    }
    .btn-discard:hover { background: rgba(239,68,68,0.25); color: rgb(239,68,68); }

    .btn-cancel {
        background: rgba(255,255,255,0.05); border-color: rgba(255,255,255,0.1);
        color: rgba(226,232,240,0.6);
    }
    .btn-cancel:hover { background: rgba(255,255,255,0.1); color: rgba(226,232,240,0.9); }
</style>
