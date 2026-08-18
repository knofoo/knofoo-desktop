<script lang="ts">
    import type { LGraphNode, LGraphCanvas } from '@comfyorg/litegraph';

    interface Props {
        node: LGraphNode;
        lgCanvas: LGraphCanvas;
        onclose: () => void;
        ondelete: () => void;
    }

    let { node, lgCanvas, onclose, ondelete }: Props = $props();

    // Accent colors for title bar
    const ACCENT_COLORS = [
        '#3b82f6', '#818cf8', '#6ee7b7', '#f472b6',
        '#fbbf24', '#f87171', '#34d399', '#e15023',
    ];

    // Body bg colors
    const BG_COLORS = [
        '#1e1e2e', '#1a1a3e', '#0f1f0f', '#2e1a1a',
        '#1a1500', '#0a1520', '#1a0e2e', '#111318',
    ];

    let accentColor  = $state('#3b82f6');
    let bgColor      = $state('#1e1e2e');
    let borderRadius = $state(8);
    let title        = $state('');
    let editingName  = $state(false);
    let inputEl      = $state<HTMLInputElement | undefined>(undefined);
    let tab          = $state<'accent' | 'bg'>('accent');

    $effect(() => {
        accentColor  = String(node.properties?.color   ?? node.color   ?? '#3b82f6');
        bgColor      = String(node.properties?.bgcolor  ?? node.bgcolor  ?? '#1e1e2e');
        borderRadius = Number(node.properties?.borderRadius ?? 8);
        title        = node.title ?? '';
    });

    $effect(() => { if (editingName) inputEl?.focus(); });

    function markChanged() {
        lgCanvas.setDirty(true, true);
        lgCanvas.graph?.change();
    }

    function applyAccent(c: string) {
        accentColor = c;
        node.properties = { ...node.properties, color: c };
        node.color = c;
        markChanged();
    }

    function applyBg(c: string) {
        bgColor = c;
        node.properties = { ...node.properties, bgcolor: c };
        node.bgcolor = c;
        markChanged();
    }

    function applyRadius(r: number) {
        borderRadius = r;
        node.properties = { ...node.properties, borderRadius: r };
        markChanged();
    }

    function commitTitle() {
        node.title = title;
        editingName = false;
        markChanged();
    }

    function onKeydown(e: KeyboardEvent) {
        if (e.key === 'Enter')  commitTitle();
        if (e.key === 'Escape') { title = node.title; editingName = false; }
    }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class="toolbar"
    style=""
    onmousedown={(e) => e.stopPropagation()}
>
    <!-- Title -->
    <div class="row">
        {#if editingName}
            <input
                bind:this={inputEl}
                class="title-input"
                bind:value={title}
                onblur={commitTitle}
                onkeydown={onKeydown}
            />
        {:else}
            <button class="title-btn" onclick={() => { editingName = true; }}>
                {node.title}
            </button>
        {/if}
    </div>

    <div class="divider"></div>

    <!-- Color tabs -->
    <div class="color-tabs">
        <button class="color-tab" class:active={tab === 'accent'} onclick={() => tab = 'accent'}>Title</button>
        <button class="color-tab" class:active={tab === 'bg'}     onclick={() => tab = 'bg'}>Body</button>
    </div>

    <div class="row row--colors">
        {#if tab === 'accent'}
            {#each ACCENT_COLORS as c}
                <button class="swatch" class:selected={accentColor === c} style="background:{c}" aria-label={c} onclick={() => applyAccent(c)}></button>
            {/each}
            <input class="color-pick" type="color" value={accentColor} aria-label="Custom title color" oninput={(e) => applyAccent((e.target as HTMLInputElement).value)} />
        {:else}
            {#each BG_COLORS as c}
                <button class="swatch" class:selected={bgColor === c} style="background:{c};border-color:rgba(255,255,255,0.15)" aria-label={c} onclick={() => applyBg(c)}></button>
            {/each}
            <input class="color-pick" type="color" value={bgColor} aria-label="Custom body color" oninput={(e) => applyBg((e.target as HTMLInputElement).value)} />
        {/if}
    </div>

    <div class="divider"></div>

    <!-- Corners -->
    <div class="row row--radius">
        <span class="label">Corners</span>
        <button class="radius-btn" class:active={borderRadius === 0}  onclick={() => applyRadius(0)}>■</button>
        <button class="radius-btn" class:active={borderRadius === 8}  onclick={() => applyRadius(8)}>▢</button>
        <button class="radius-btn" class:active={borderRadius === 16} onclick={() => applyRadius(16)}>⬭</button>
    </div>

    <div class="divider"></div>

    <!-- Actions -->
    <div class="row row--actions">
        <button class="action-btn danger" onclick={ondelete}>Delete</button>
        <button class="action-btn" onclick={onclose}>Done</button>
    </div>
</div>

<style>
    .toolbar {
        z-index: 100;
        background: #16162a;
        border: 1px solid rgba(255,255,255,0.12);
        border-radius: 8px;
        box-shadow: 0 8px 32px rgba(0,0,0,0.6);
        padding: 6px;
        display: flex;
        flex-direction: column;
        gap: 4px;
        min-width: 210px;
        pointer-events: auto;
    }

    .row { display: flex; align-items: center; gap: 4px; }
    .divider { height: 1px; background: rgba(255,255,255,0.07); margin: 2px 0; }

    .title-btn {
        flex: 1;
        background: rgba(255,255,255,0.04);
        border: 1px solid rgba(255,255,255,0.08);
        border-radius: 4px;
        color: rgba(226,232,240,0.85);
        font-size: 0.88rem;
        padding: 4px 8px;
        text-align: left;
        cursor: text;
    }

    .title-input {
        flex: 1;
        background: rgba(255,255,255,0.06);
        border: 1px solid rgba(225,80,35,0.6);
        border-radius: 4px;
        color: rgba(226,232,240,0.95);
        font-size: 0.88rem;
        padding: 4px 8px;
        outline: none;
    }

    .color-tabs {
        display: flex;
        gap: 2px;
        margin-bottom: 2px;
    }

    .color-tab {
        flex: 1;
        padding: 3px 0;
        background: rgba(255,255,255,0.04);
        border: 1px solid rgba(255,255,255,0.08);
        border-radius: 4px;
        color: rgba(226,232,240,0.4);
        font-size: 0.75rem;
        cursor: pointer;
        transition: all 0.1s;
    }

    .color-tab.active {
        background: rgba(225,80,35,0.18);
        border-color: rgba(225,80,35,0.4);
        color: rgba(226,232,240,0.9);
    }

    .row--colors { flex-wrap: wrap; gap: 5px; padding: 2px 0; }

    .swatch {
        width: 18px; height: 18px;
        border-radius: 4px;
        border: 2px solid transparent;
        cursor: pointer;
        padding: 0;
        transition: border-color 0.1s, transform 0.1s;
    }
    .swatch:hover    { transform: scale(1.15); }
    .swatch.selected { border-color: #fff; }

    .color-pick {
        width: 18px; height: 18px;
        border: none; border-radius: 4px;
        padding: 0; cursor: pointer; background: none;
    }

    .row--radius { gap: 6px; }

    .label { font-size: 0.75rem; color: rgba(226,232,240,0.35); margin-right: 2px; }

    .radius-btn {
        width: 24px; height: 24px;
        background: rgba(255,255,255,0.04);
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 4px;
        color: rgba(226,232,240,0.5);
        font-size: 0.85rem;
        cursor: pointer;
        display: flex; align-items: center; justify-content: center;
        padding: 0;
        transition: all 0.1s;
    }
    .radius-btn:hover  { background: rgba(255,255,255,0.08); color: rgba(226,232,240,0.9); }
    .radius-btn.active { background: rgba(225,80,35,0.2); border-color: rgba(225,80,35,0.5); color: #e15023; }

    .row--actions { justify-content: flex-end; }

    .action-btn {
        padding: 4px 12px;
        background: rgba(255,255,255,0.05);
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 4px;
        color: rgba(226,232,240,0.7);
        font-size: 0.82rem;
        cursor: pointer;
        transition: all 0.1s;
    }
    .action-btn:hover  { background: rgba(255,255,255,0.1); color: rgba(226,232,240,0.95); }
    .action-btn.danger { color: rgba(239,68,68,0.8); }
    .action-btn.danger:hover { background: rgba(239,68,68,0.15); color: rgb(239,68,68); border-color: rgba(239,68,68,0.3); }
</style>
