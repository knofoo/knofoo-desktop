<script lang="ts">
    import { graphStore } from '$lib/stores/graphStore.svelte';
    import FileTree from './FileTree.svelte';

    type FileEntry = { name: string; path: string; is_dir: boolean; children?: FileEntry[] };
    type Section = 'graph' | 'module' | 'vault';

    interface Props {
        entries: FileEntry[];
        section: Section;
        depth?: number;
        expandedPaths?: Set<string>;
        focusPath?: string | null;
        onContextMenu: (e: MouseEvent, entry: FileEntry) => void;
        onOpen: (entry: FileEntry) => void;
        onToggle?: (path: string, open: boolean) => void;
    }

    let { entries, section, depth = 0, expandedPaths = new Set(), focusPath = null, onContextMenu, onOpen, onToggle }: Props = $props();

    function toggle(entry: FileEntry) {
        const next = !expandedPaths.has(entry.path);
        onToggle?.(entry.path, next);
    }

    function displayName(entry: FileEntry) {
        if (entry.is_dir) return entry.name;
        const dot = entry.name.lastIndexOf('.');
        return dot > 0 ? entry.name.slice(0, dot) : entry.name;
    }

    function extLabel(entry: FileEntry) {
        if (entry.is_dir) return '';
        const dot = entry.name.lastIndexOf('.');
        return dot > 0 ? entry.name.slice(dot + 1).toUpperCase() : '';
    }

    function isActive(entry: FileEntry) {
        return !entry.is_dir && graphStore.tabs.some(t => t.path === entry.path && t.id === graphStore.active);
    }

    function isFocused(entry: FileEntry) {
        return focusPath === entry.path;
    }
</script>

<ul class="file-list">
    {#each entries as entry}
        {@const indent = depth * 12}
        {@const isExpanded = expandedPaths.has(entry.path)}
        <li class="file-item-wrap">
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <button
                class="file-item"
                class:is-dir={entry.is_dir}
                class:active={isActive(entry)}
                class:focused={isFocused(entry)}
                style="padding-left: {12 + indent}px"
                draggable={!entry.is_dir}
                ondragstart={(e) => {
                    if (entry.is_dir) return;
                    e.dataTransfer!.effectAllowed = 'copy';
                    e.dataTransfer!.setData('application/knofoo-file', JSON.stringify({ path: entry.path, section, name: displayName(entry) }));
                }}
                onclick={() => entry.is_dir ? toggle(entry) : onOpen(entry)}
                oncontextmenu={(e) => { e.stopPropagation(); onContextMenu(e, entry); }}
            >
                <span class="file-icon">
                    {#if entry.is_dir}
                        {isExpanded ? '▼' : '▶'}
                    {:else if section === 'graph'}◇
                    {:else if section === 'module'}⊟
                    {:else}·{/if}
                </span>
                <span class="file-name">{displayName(entry)}</span>
                {#if !entry.is_dir && extLabel(entry)}<span class="file-ext">{extLabel(entry)}</span>{/if}
            </button>

            {#if entry.is_dir && isExpanded && entry.children}
                <FileTree
                    entries={entry.children}
                    {section}
                    depth={depth + 1}
                    {expandedPaths}
                    {focusPath}
                    {onContextMenu}
                    {onOpen}
                    {onToggle}
                />
            {/if}
        </li>
    {/each}
</ul>

<style>
    .file-list { list-style: none; padding: 0; margin: 0; }
    .file-item-wrap { margin: 0 4px; }

    .file-item {
        display: flex;
        align-items: center;
        gap: 6px;
        padding-top: 5px;
        padding-bottom: 5px;
        padding-right: 10px;
        cursor: pointer;
        border-radius: 4px;
        width: 100%;
        background: none;
        border: none;
        text-align: left;
        outline: none;
    }

    .file-item:hover   { background: rgba(255,255,255,0.05); }
    .file-item.active  { background: rgba(225,80,35,0.12); }
    .file-item.active .file-name { color: rgba(226,232,240,0.95); }
    .file-item.active .file-icon { color: rgba(225,80,35,0.9); }
    .file-item.focused { background: rgba(225,80,35,0.08); outline: 1px solid rgba(225,80,35,0.35); }
    .file-item.focused .file-name { color: rgba(226,232,240,0.95); }

    .file-icon {
        font-size: 0.72rem;
        color: rgba(225,80,35,0.5);
        flex-shrink: 0;
        width: 12px;
        text-align: center;
    }

    .file-item.is-dir .file-icon { font-size: 0.55rem; color: rgba(226,232,240,0.3); }

    .file-name {
        font-size: 0.92rem;
        color: rgba(226,232,240,0.75);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        flex: 1;
        min-width: 0;
    }

    .file-item.is-dir .file-name { color: rgba(226,232,240,0.6); }

    .file-ext {
        font-size: 0.72rem;
        color: rgba(226,232,240,0.2);
        font-family: monospace;
        white-space: nowrap;
        flex-shrink: 0;
    }
</style>
