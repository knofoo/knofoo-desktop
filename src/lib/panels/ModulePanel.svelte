<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import Dropdown from '$lib/components/Dropdown.svelte';

    interface ValidatorBlock {
        id: string;
        type: 'qa';
        question: string;
        answer: string;
        hints: string[];
        strategy: 'regex' | 'nlp' | 'choice' | 'hash';
        blocking: boolean;
        weight: number;
    }

    interface ContentBlock {
        id: string;
        type: 'markdown';
        content: string;
    }

    type Block = ContentBlock | ValidatorBlock;

    type ModMode = 'collapsed' | 'split' | 'expanded';

    interface Props {
        expanded: boolean;
        visible: boolean;
        mode: ModMode;
        path: string | null;
        onmodechange: (next: ModMode) => void;
        onclose: () => void;
        onpathchange?: (oldPath: string, newPath: string) => void;
    }

    let { expanded, visible, mode, path, onmodechange, onclose, onpathchange }: Props = $props();

    let blocks    = $state<Block[]>([]);
    let title     = $state('');
    let desc      = $state('');
    let fileBase  = $state('');  // editable filename without .json
    let editingTitle = $state(false);
    let titleDraft   = $state('');
    let renaming  = $state(false);
    let renameError = $state<string | null>(null);
    let dirty     = $state(false);
    let saving    = $state(false);
    let viewMode  = $state<'edit' | 'preview'>('edit');

    // Initial filename derived from path on load.
    function deriveFileBase(p: string | null): string {
        if (!p) return '';
        const base = p.split(/[\\/]/).pop() ?? '';
        return base.replace(/\.json$/i, '');
    }

    // Header shows the filename only — title and description are body fields
    // that drive the node's appearance in the graph.
    const headerTitle = $derived(fileBase || 'Module');

    // Load module when path changes
    $effect(() => {
        if (path) {
            fileBase = deriveFileBase(path);
            loadModule(path);
        } else {
            blocks = []; title = ''; desc = ''; fileBase = ''; dirty = false;
        }
    });

    async function loadModule(p: string) {
        try {
            const raw = await invoke<string>('read_file', { path: p });
            const data = JSON.parse(raw);
            title  = data?.meta?.title ?? '';
            desc   = data?.meta?.description ?? '';
            blocks = (data?.graph?.nodes ?? []).map((n: any): Block => {
                if (n.type === 'qa') {
                    return {
                        id:       n.id,
                        type:     'qa',
                        question: n.data?.question ?? '',
                        answer:   n.data?.answer   ?? '',
                        hints:    n.data?.hints     ?? [],
                        strategy: n.data?.strategy  ?? 'regex',
                        blocking: n.data?.blocking  ?? true,
                        weight:   n.proof?.weight   ?? 1.0,
                    };
                }
                return {
                    id:      n.id,
                    type:    'markdown',
                    content: n.data?.content ?? '',
                };
            });
            dirty = false;
        } catch (e) { console.error('load module failed:', e); }
    }

    async function save() {
        if (!path) return;
        saving = true;
        try {
            const raw = await invoke<string>('read_file', { path });
            const data = JSON.parse(raw);
            data.meta = { ...(data.meta ?? {}), title, description: desc, updated: Math.floor(Date.now() / 1000) };
            data.graph = {
                nodes: blocks.map((b, i) => ({
                    id:       b.id,
                    type:     b.type === 'qa' ? 'qa' : 'markdown',
                    position: data.graph?.nodes?.[i]?.position ?? { x: i * 300, y: 0 },
                    size:     data.graph?.nodes?.[i]?.size     ?? { w: 260, h: b.type === 'qa' ? 120 : 160 },
                    meta:     { title: b.type === 'markdown' ? 'Content' : 'Validator', tags: [], locked: false, visible: true },
                    data:     b.type === 'qa'
                        ? { question: b.question, answer: b.answer, hints: b.hints, strategy: b.strategy, blocking: b.blocking }
                        : { content: b.content },
                    connections: { requires: [], unlocks: [] },
                    proof:    { strategy: b.type === 'qa' ? b.strategy : 'none', weight: b.type === 'qa' ? b.weight : 1.0 },
                })),
                edges: data.graph?.edges ?? [],
            };
            await invoke('write_file', { path, content: JSON.stringify(data, null, 2) });
            dirty = false;
            // Tell open graphs to update any module node referencing this path
            // so the title / description shown on the node reflect the saved meta.
            window.dispatchEvent(new CustomEvent('knofoo:module-meta-updated', {
                detail: { path, title, description: desc },
            }));
        } catch (e) { console.error('save module failed:', e); }
        saving = false;
    }

    function startEditTitle() {
        if (!path) return;
        titleDraft = fileBase;
        editingTitle = true;
    }

    async function commitTitle() {
        if (!editingTitle) return;
        editingTitle = false;
        const trimmed = titleDraft.trim();
        renameError = null;
        if (!path || !trimmed || trimmed === fileBase) return;
        if (/[\\\/:\*\?"<>\|]/.test(trimmed)) { renameError = 'Invalid characters'; return; }

        renaming = true;
        const sepIdx = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
        const sep = path[sepIdx] ?? '/';
        const dir = path.substring(0, sepIdx);
        const newPath = `${dir}${sep}${trimmed}.json`;
        console.log('[module rename]', path, '->', newPath);
        try {
            const exists = await invoke<boolean>('exists_path', { path: newPath });
            if (exists) {
                renameError = 'A file with that name already exists';
                renaming = false;
                return;
            }
            await invoke('rename_path', { src: path, dst: newPath });
            const old = path;
            fileBase = trimmed;
            onpathchange?.(old, newPath);
            // Tell explorer panels to refresh — fs watch can be flaky in Tauri webview.
            window.dispatchEvent(new CustomEvent('knofoo:reload-modules'));
        } catch (e) {
            renameError = `Rename failed: ${String(e)}`;
        }
        renaming = false;
    }

    function cancelEditTitle() {
        editingTitle = false;
        titleDraft = '';
        renameError = null;
    }

    function addBlock(type: 'markdown' | 'qa') {
        const id = `node_${Date.now()}`;
        if (type === 'markdown') {
            blocks = [...blocks, { id, type: 'markdown', content: '' }];
        } else {
            blocks = [...blocks, { id, type: 'qa', question: '', answer: '', hints: [], strategy: 'regex', blocking: true, weight: 1.0 }];
        }
        dirty = true;
    }

    function removeBlock(id: string) {
        blocks = blocks.filter(b => b.id !== id);
        dirty = true;
    }

    function moveBlock(idx: number, dir: -1 | 1) {
        const next = idx + dir;
        if (next < 0 || next >= blocks.length) return;
        const arr = [...blocks];
        [arr[idx], arr[next]] = [arr[next], arr[idx]];
        blocks = arr;
        dirty = true;
    }

    function updateBlock(id: string, patch: Partial<Block>) {
        blocks = blocks.map(b => b.id === id ? { ...b, ...patch } as Block : b);
        dirty = true;
    }

    function updateHints(id: string, raw: string) {
        const hints = raw.split(',').map(s => s.trim()).filter(Boolean);
        updateBlock(id, { hints } as any);
    }

    // Ctrl/Cmd+S saves the open module while the panel is visible.
    $effect(() => {
        if (!visible || !path) return;
        function onKd(e: KeyboardEvent) {
            if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
                e.preventDefault();
                if (dirty) save();
            }
        }
        window.addEventListener('keydown', onKd);
        return () => window.removeEventListener('keydown', onKd);
    });
</script>

{#if visible}
<div class="module-panel" class:expanded>
    <div class="header">
        <span class="header-icon">⊞</span>
        {#if editingTitle}
            <input
                class="header-title-input"
                value={titleDraft}
                disabled={renaming}
                autofocus
                oninput={(e) => { titleDraft = e.currentTarget.value; renameError = null; }}
                onblur={commitTitle}
                onkeydown={(e) => {
                    if (e.key === 'Enter')  (e.currentTarget as HTMLInputElement).blur();
                    if (e.key === 'Escape') cancelEditTitle();
                }}
            />
        {:else}
            <span class="header-title editable" title="Click to rename" onclick={startEditTitle}
                  role="button" tabindex="0"
                  onkeydown={(e) => { if (e.key === 'Enter') startEditTitle(); }}>
                {headerTitle}
                <span class="edit-icon">✎</span>
            </span>
        {/if}
        {#if renameError}
            <span class="rename-error-inline" title={renameError}>!</span>
        {/if}
        {#if dirty}
            <span class="dirty-dot" title="Unsaved">●</span>
        {/if}
        <div class="header-modes">
            <button class="mode-btn" class:active={viewMode === 'edit'}    onclick={() => viewMode = 'edit'}>Edit</button>
            <button class="mode-btn" class:active={viewMode === 'preview'} onclick={() => viewMode = 'preview'}>Preview</button>
        </div>
        <button class="save-btn" onclick={save} disabled={!dirty || saving} title="Save (Ctrl+S)">
            {saving ? '…' : 'Save'}
        </button>
        <button class="icon-btn"
                onclick={() => onmodechange(mode === 'expanded' ? 'split' : 'expanded')}
                title={mode === 'expanded' ? 'Restore split' : 'Expand to fill'}>
            {mode === 'expanded' ? '▤' : '▣'}
        </button>
        <button class="icon-btn"
                onclick={() => onmodechange(mode === 'collapsed' ? 'split' : 'collapsed')}
                title={mode === 'collapsed' ? 'Expand' : 'Collapse'}>
            {mode === 'collapsed' ? '⌃' : '⌄'}
        </button>
        <button class="icon-btn close-btn" onclick={onclose} title="Close">×</button>
    </div>

    {#if expanded}
        <div class="content">
            {#if !path}
                <div class="empty">Double-click a module node to open its content here</div>
            {:else if viewMode === 'edit'}
                <div class="editor">
                    <div class="meta-fields">
                        <input
                            class="meta-title"
                            placeholder="Title (shown on the graph node)"
                            value={title}
                            oninput={(e) => { title = e.currentTarget.value; dirty = true; }}
                        />
                        <textarea
                            class="meta-desc"
                            rows="2"
                            placeholder="Description (shown under title on the graph node)"
                            value={desc}
                            oninput={(e) => { desc = e.currentTarget.value; dirty = true; }}
                        ></textarea>
                    </div>

                    <div class="blocks">
                        {#each blocks as block, i (block.id)}
                            <div class="block" class:block-qa={block.type === 'qa'}>
                                <div class="block-header">
                                    <span class="block-type">{block.type === 'qa' ? '⬡ Validator' : '◇ Content'}</span>
                                    <div class="block-actions">
                                        <button class="act-btn" onclick={() => moveBlock(i, -1)} disabled={i === 0} title="Move up">↑</button>
                                        <button class="act-btn" onclick={() => moveBlock(i, 1)} disabled={i === blocks.length - 1} title="Move down">↓</button>
                                        <button class="act-btn danger" onclick={() => removeBlock(block.id)} title="Remove">×</button>
                                    </div>
                                </div>

                                {#if block.type === 'markdown'}
                                    <textarea
                                        class="block-textarea"
                                        rows="5"
                                        placeholder="Write markdown content…"
                                        value={block.content}
                                        oninput={(e) => updateBlock(block.id, { content: e.currentTarget.value })}
                                    ></textarea>
                                {:else}
                                    <div class="qa-fields">
                                        <label for="{block.id}-q">Question</label>
                                        <textarea
                                            id="{block.id}-q"
                                            class="block-textarea"
                                            rows="2"
                                            placeholder="What is the question?"
                                            value={block.question}
                                            oninput={(e) => updateBlock(block.id, { question: e.currentTarget.value })}
                                        ></textarea>
                                        <label for="{block.id}-a">Answer</label>
                                        <input
                                            id="{block.id}-a"
                                            class="block-input"
                                            placeholder="Expected answer…"
                                            value={block.answer}
                                            oninput={(e) => updateBlock(block.id, { answer: e.currentTarget.value })}
                                        />
                                        <label for="{block.id}-h">Hints <span class="label-hint">(comma separated)</span></label>
                                        <input
                                            id="{block.id}-h"
                                            class="block-input"
                                            placeholder="hint 1, hint 2…"
                                            value={block.hints.join(', ')}
                                            oninput={(e) => updateHints(block.id, e.currentTarget.value)}
                                        />
                                        <div class="qa-row">
                                            <div class="qa-field">
                                                <label for="{block.id}-s">Strategy</label>
                                                <Dropdown
                                                    value={block.strategy}
                                                    onchange={(v) => updateBlock(block.id, { strategy: v as any })}
                                                    options={[
                                                        { value: 'regex',  label: 'regex'  },
                                                        { value: 'nlp',    label: 'nlp'    },
                                                        { value: 'choice', label: 'choice' },
                                                        { value: 'hash',   label: 'hash'   },
                                                    ]}
                                                />
                                            </div>
                                            <div class="qa-field">
                                                <label for="{block.id}-b">Blocking</label>
                                                <Dropdown
                                                    value={block.blocking ? 'true' : 'false'}
                                                    onchange={(v) => updateBlock(block.id, { blocking: v === 'true' })}
                                                    options={[
                                                        { value: 'true',  label: 'Yes' },
                                                        { value: 'false', label: 'No'  },
                                                    ]}
                                                />
                                            </div>
                                            <div class="qa-field">
                                                <label for="{block.id}-w">Weight</label>
                                                <input
                                                    id="{block.id}-w"
                                                    class="block-input"
                                                    type="number"
                                                    min="0"
                                                    max="10"
                                                    step="0.1"
                                                    value={block.weight}
                                                    oninput={(e) => updateBlock(block.id, { weight: parseFloat(e.currentTarget.value) || 1 })}
                                                />
                                            </div>
                                        </div>
                                    </div>
                                {/if}
                            </div>
                        {/each}
                    </div>

                    <div class="add-buttons">
                        <button class="add-btn" onclick={() => addBlock('markdown')}>+ Content</button>
                        <button class="add-btn add-btn-qa" onclick={() => addBlock('qa')}>+ Validator</button>
                    </div>
                </div>
            {:else}
                <div class="preview">
                    <h1 class="preview-title">{title || 'Untitled'}</h1>
                    {#if desc}<p class="preview-desc">{desc}</p>{/if}
                    <hr class="preview-hr" />
                    {#each blocks as block (block.id)}
                        {#if block.type === 'markdown'}
                            <div class="preview-content">{block.content}</div>
                        {:else}
                            <div class="preview-qa">
                                <div class="qa-badge">⬡ {block.strategy}</div>
                                <div class="qa-question">{block.question || 'No question set'}</div>
                                <details class="qa-answer-wrap">
                                    <summary>Show answer</summary>
                                    <div class="qa-answer">{block.answer || '—'}</div>
                                    {#if block.hints.length > 0}
                                        <div class="qa-hints">Hints: {block.hints.join(' · ')}</div>
                                    {/if}
                                </details>
                            </div>
                        {/if}
                    {/each}
                    {#if blocks.length === 0}
                        <div class="preview-empty">No blocks yet. Switch to Edit to add content.</div>
                    {/if}
                </div>
            {/if}
        </div>
    {/if}
</div>
{/if}

<style>
    .module-panel {
        display: flex;
        flex-direction: column;
        flex: 1;
        min-height: 0;
        background: #0a0a12;
        overflow: hidden;
    }

    .header {
        display: flex;
        align-items: center;
        gap: 4px;
        padding: 0 8px 0 12px;
        height: 30px;
        flex-shrink: 0;
        user-select: none;
    }

    .header-icon { font-size: 0.75rem; color: rgba(14,165,233,0.7); }

    .header-title {
        font-size: 0.8rem;
        font-weight: 600;
        color: rgba(226,232,240,0.5);
        flex: 1;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        display: flex;
        align-items: center;
        gap: 6px;
    }
    .header-title.editable {
        cursor: pointer;
        padding: 2px 6px;
        margin-left: -6px;
        border-radius: 3px;
        transition: background 0.1s, color 0.1s;
    }
    .header-title.editable:hover {
        background: rgba(255,255,255,0.05);
        color: rgba(226,232,240,0.85);
    }
    .header-title .edit-icon {
        font-size: 0.7rem;
        opacity: 0;
        transition: opacity 0.15s;
        color: rgba(14,165,233,0.7);
    }
    .header-title.editable:hover .edit-icon { opacity: 1; }

    .header-title-input {
        flex: 1;
        font-size: 0.8rem;
        font-weight: 600;
        font-family: inherit;
        background: rgba(255,255,255,0.06);
        border: 1px solid rgba(14,165,233,0.45);
        border-radius: 3px;
        color: #e2e8f0;
        padding: 2px 6px;
        margin-left: -6px;
        outline: none;
    }

    .rename-error-inline {
        color: #f472b6;
        font-weight: 700;
        font-size: 0.85rem;
        padding: 0 4px;
    }

    .dirty-dot { font-size: 0.5rem; color: #fbbf24; }

    .header-modes {
        display: flex;
        gap: 2px;
        background: rgba(255,255,255,0.05);
        border-radius: 4px;
        padding: 2px;
    }

    .mode-btn {
        background: none;
        border: none;
        color: rgba(226,232,240,0.35);
        font-size: 0.72rem;
        padding: 1px 7px;
        border-radius: 3px;
        cursor: pointer;
        transition: all 0.1s;
    }
    .mode-btn.active { background: rgba(14,165,233,0.2); color: rgba(14,165,233,0.9); }
    .mode-btn:hover:not(.active) { color: rgba(226,232,240,0.6); }

    .icon-btn {
        background: none; border: none;
        color: rgba(226,232,240,0.3);
        cursor: pointer; font-size: 0.9rem;
        width: 20px; height: 20px;
        display: flex; align-items: center; justify-content: center;
        border-radius: 3px; padding: 0; flex-shrink: 0;
        transition: color 0.1s, background 0.1s;
    }
    .icon-btn:hover:not(:disabled) { color: rgba(226,232,240,0.7); background: rgba(255,255,255,0.05); }
    .icon-btn:disabled { opacity: 0.3; cursor: default; }
    .icon-btn.active {
        color: rgba(14,165,233,0.95);
        background: rgba(14,165,233,0.12);
    }
    .size-btns {
        display: flex; gap: 1px;
        background: rgba(255,255,255,0.04);
        border-radius: 4px;
        padding: 1px;
    }
    .close-btn:hover { color: rgba(239,68,68,0.8); background: rgba(239,68,68,0.1); }

    .save-btn {
        padding: 3px 12px;
        background: rgba(14,165,233,0.15);
        border: 1px solid rgba(14,165,233,0.35);
        border-radius: 4px;
        color: rgba(14,165,233,0.9);
        font-size: 0.78rem;
        font-family: monospace;
        cursor: pointer;
        flex-shrink: 0;
    }
    .save-btn:hover:not(:disabled) {
        background: rgba(14,165,233,0.28);
        color: #e2e8f0;
    }
    .save-btn:disabled {
        opacity: 0.35;
        cursor: default;
    }

    .content {
        flex: 1; min-height: 0; overflow: auto;
        display: flex; flex-direction: column;
    }

    .empty {
        display: flex; align-items: center; justify-content: center;
        flex: 1;
        font-size: 0.82rem; font-family: monospace;
        color: rgba(226,232,240,0.15);
    }

    /* ── Editor ── */
    .editor {
        display: flex; flex-direction: column;
        padding: 14px 16px; gap: 12px;
        flex: 1;
    }

    .meta-fields { display: flex; flex-direction: column; gap: 6px; }

    .filename-row {
        display: flex; align-items: center; gap: 6px;
    }
    .filename-label {
        font-size: 0.7rem; font-weight: 600;
        color: rgba(226,232,240,0.35);
        text-transform: uppercase; letter-spacing: 0.06em;
        flex-shrink: 0;
    }
    .filename-input {
        flex: 1;
        background: rgba(255,255,255,0.04);
        border: 1px solid rgba(255,255,255,0.08);
        border-radius: 4px; color: rgba(226,232,240,0.9);
        font-family: monospace; font-size: 0.85rem;
        padding: 4px 8px;
    }
    .filename-input:focus { outline: none; border-color: rgba(14,165,233,0.4); }
    .filename-ext {
        font-family: monospace; font-size: 0.85rem;
        color: rgba(226,232,240,0.3);
    }
    .rename-error {
        font-size: 0.78rem; color: #f472b6;
        font-family: monospace;
    }

    .meta-title {
        background: rgba(255,255,255,0.04);
        border: 1px solid rgba(255,255,255,0.08);
        border-radius: 5px; color: rgba(226,232,240,0.9);
        font-size: 1rem; font-weight: 600;
        padding: 6px 10px;
    }
    .meta-title:focus { outline: none; border-color: rgba(14,165,233,0.4); }

    .meta-desc {
        background: rgba(255,255,255,0.03);
        border: 1px solid rgba(255,255,255,0.07);
        border-radius: 5px; color: rgba(226,232,240,0.6);
        font-size: 0.85rem; padding: 6px 10px;
        resize: none; font-family: inherit;
    }
    .meta-desc:focus { outline: none; border-color: rgba(14,165,233,0.3); }

    .blocks { display: flex; flex-direction: column; gap: 10px; }

    .block {
        background: rgba(255,255,255,0.03);
        border: 1px solid rgba(255,255,255,0.07);
        border-radius: 7px;
        padding: 10px 12px;
        display: flex; flex-direction: column; gap: 8px;
    }
    .block-qa { border-color: rgba(129,140,248,0.2); background: rgba(129,140,248,0.04); }

    .block-header {
        display: flex; align-items: center; justify-content: space-between;
    }
    .block-type {
        font-size: 0.72rem; font-weight: 600;
        color: rgba(226,232,240,0.3);
        text-transform: uppercase; letter-spacing: 0.06em;
    }
    .block-actions { display: flex; gap: 3px; }
    .act-btn {
        background: none; border: 1px solid rgba(255,255,255,0.08);
        border-radius: 3px; color: rgba(226,232,240,0.3);
        font-size: 0.8rem; width: 20px; height: 20px;
        cursor: pointer; padding: 0;
        display: flex; align-items: center; justify-content: center;
        transition: all 0.1s;
    }
    .act-btn:hover:not(:disabled) { color: rgba(226,232,240,0.8); background: rgba(255,255,255,0.06); }
    .act-btn:disabled { opacity: 0.25; cursor: default; }
    .act-btn.danger:hover { color: rgba(239,68,68,0.8); background: rgba(239,68,68,0.1); border-color: rgba(239,68,68,0.3); }

    .block-textarea {
        background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.07);
        border-radius: 5px; color: rgba(226,232,240,0.8);
        font-size: 0.85rem; font-family: monospace;
        padding: 8px 10px; resize: vertical; width: 100%;
    }
    .block-textarea:focus { outline: none; border-color: rgba(14,165,233,0.35); }

    .block-input {
        background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.07);
        border-radius: 5px; color: rgba(226,232,240,0.8);
        font-size: 0.85rem; padding: 6px 10px; width: 100%;
    }
    .block-input:focus { outline: none; border-color: rgba(129,140,248,0.4); }

    .block-select {
        background: rgba(0,0,0,0.4); border: 1px solid rgba(255,255,255,0.08);
        border-radius: 4px; color: rgba(226,232,240,0.7);
        font-size: 0.82rem; padding: 4px 6px; width: 100%;
    }

    .qa-fields { display: flex; flex-direction: column; gap: 6px; }

    .qa-fields label {
        font-size: 0.72rem; font-weight: 600;
        color: rgba(129,140,248,0.6);
        text-transform: uppercase; letter-spacing: 0.05em;
    }
    .label-hint { font-weight: 400; text-transform: none; color: rgba(226,232,240,0.2); }

    .qa-row { display: flex; gap: 8px; }
    .qa-field { flex: 1; display: flex; flex-direction: column; gap: 4px; }

    .add-buttons { display: flex; gap: 8px; padding-top: 4px; }

    .add-btn {
        padding: 5px 14px; background: rgba(14,165,233,0.12);
        border: 1px solid rgba(14,165,233,0.25); border-radius: 5px;
        color: rgba(14,165,233,0.8); font-size: 0.82rem; cursor: pointer;
        transition: all 0.1s;
    }
    .add-btn:hover { background: rgba(14,165,233,0.22); color: rgba(14,165,233,1); }
    .add-btn-qa {
        background: rgba(129,140,248,0.1); border-color: rgba(129,140,248,0.25);
        color: rgba(129,140,248,0.8);
    }
    .add-btn-qa:hover { background: rgba(129,140,248,0.2); color: rgba(129,140,248,1); }

    /* ── Preview ── */
    .preview {
        padding: 20px 24px;
        color: rgba(226,232,240,0.8);
        font-size: 0.92rem; line-height: 1.7;
        flex: 1;
    }
    .preview-title { font-size: 1.5rem; font-weight: 700; margin: 0 0 6px; color: rgba(226,232,240,0.95); }
    .preview-desc { margin: 0 0 12px; color: rgba(226,232,240,0.45); font-size: 0.9rem; }
    .preview-hr { border: none; border-top: 1px solid rgba(255,255,255,0.07); margin: 14px 0; }
    .preview-content { white-space: pre-wrap; margin-bottom: 16px; }
    .preview-empty { color: rgba(226,232,240,0.15); font-family: monospace; font-size: 0.85rem; }

    .preview-qa {
        background: rgba(129,140,248,0.06);
        border: 1px solid rgba(129,140,248,0.2);
        border-radius: 8px; padding: 12px 14px;
        margin-bottom: 14px;
    }
    .qa-badge {
        font-size: 0.7rem; font-weight: 600; text-transform: uppercase;
        color: rgba(129,140,248,0.6); letter-spacing: 0.07em; margin-bottom: 6px;
    }
    .qa-question { font-weight: 600; color: rgba(226,232,240,0.9); margin-bottom: 8px; }
    .qa-answer-wrap summary {
        cursor: pointer; font-size: 0.8rem;
        color: rgba(129,140,248,0.6); user-select: none;
    }
    .qa-answer { margin-top: 6px; color: rgba(134,239,172,0.8); font-family: monospace; font-size: 0.85rem; }
    .qa-hints { margin-top: 4px; font-size: 0.78rem; color: rgba(226,232,240,0.3); }
</style>
