<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { watchImmediate } from '@tauri-apps/plugin-fs';
    import { vault } from '$lib/stores/vaultStore.svelte';
    import { graphStore } from '$lib/stores/graphStore.svelte';
    import FileTree from './FileTree.svelte';

    type FileEntry = { name: string; path: string; is_dir: boolean; children?: FileEntry[] };
    type Section = 'graph' | 'module' | 'vault';

    const folderName  = $derived(vault.path ? vault.path.split(/[\\/]/).filter(Boolean).at(-1) : null);
    const graphsPath  = $derived(vault.path ? vault.resolvePath(vault.config.paths.graphs)  : null);
    const modulesPath = $derived(vault.path ? vault.resolvePath(vault.config.paths.modules) : null);

async function uniquePath(dir: string, base: string, ext: string): Promise<string> {
        const name = ext ? `${base}.${ext}` : base;
        if (!await invoke<boolean>('exists_path', { path: `${dir}/${name}` })) return `${dir}/${name}`;
        let n = 1;
        while (true) {
            const candidate = ext ? `${dir}/${base} ${n}.${ext}` : `${dir}/${base} ${n}`;
            if (!await invoke<boolean>('exists_path', { path: candidate })) return candidate;
            n++;
        }
    }

    let graphs     = $state<FileEntry[]>([]);
    let modules    = $state<FileEntry[]>([]);
    let vaultFiles = $state<FileEntry[]>([]);

    // Expanded folder paths — lifted out of FileTree so reloads don't reset them
    let graphExpanded  = $state(new Set<string>());
    let moduleExpanded = $state(new Set<string>());
    let vaultExpanded  = $state(new Set<string>());

    function expandedFor(section: Section) {
        if (section === 'graph')  return graphExpanded;
        if (section === 'module') return moduleExpanded;
        return vaultExpanded;
    }

    function onToggle(section: Section, path: string, open: boolean) {
        const s = new Set(expandedFor(section));
        open ? s.add(path) : s.delete(path);
        if (section === 'graph')  graphExpanded  = s;
        if (section === 'module') moduleExpanded = s;
        if (section === 'vault')  vaultExpanded  = s;
    }

    let focusPath = $state<string | null>(null);

    function setFocus(section: Section, path: string) {
        // Auto-expand parent folder
        const sep = path.includes('/') ? '/' : '\\';
        const parent = path.substring(0, path.lastIndexOf(sep));
        if (parent) {
            const s = new Set(expandedFor(section));
            s.add(parent);
            if (section === 'graph')  graphExpanded  = s;
            if (section === 'module') moduleExpanded = s;
            if (section === 'vault')  vaultExpanded  = s;
        }
        focusPath = path;
        setTimeout(() => { focusPath = null; }, 2000);
    }

    async function loadGraphs() {
        if (!graphsPath) return;
        try { graphs = await invoke<FileEntry[]>('list_dir_recursive', { vaultPath: graphsPath }); } catch { graphs = []; }
    }
    async function loadModules() {
        if (!modulesPath) return;
        try { modules = await invoke<FileEntry[]>('list_dir_recursive', { vaultPath: modulesPath }); } catch { modules = []; }
    }
    async function loadVaultFiles() {
        const p = vault.path; if (!p) return;
        try {
            const all = await invoke<FileEntry[]>('list_dir_recursive', { vaultPath: p });
            vaultFiles = all.filter(e => e.name !== '.knofoo');
        } catch { vaultFiles = []; }
    }

    async function reload(section: Section) {
        if (section === 'graph')  await loadGraphs();
        if (section === 'module') await loadModules();
        if (section === 'vault')  await loadVaultFiles();
    }

    function debounce(fn: () => void, ms: number) {
        let t: ReturnType<typeof setTimeout>;
        return () => { clearTimeout(t); t = setTimeout(fn, ms); };
    }

    $effect(() => {
        if (!vault.path) return;
        invoke('init_vault', { path: vault.path }).catch(() => {}).then(() => {
            loadGraphs(); loadModules(); loadVaultFiles();
        });
    });

    $effect(() => {
        if (!graphsPath) return;
        let unwatch: (() => void) | undefined;
        watchImmediate(graphsPath, debounce(loadGraphs, 80), { recursive: true }).then(u => { unwatch = u; }).catch(() => {});
        return () => unwatch?.();
    });

    $effect(() => {
        if (!modulesPath) return;
        let unwatch: (() => void) | undefined;
        watchImmediate(modulesPath, debounce(loadModules, 80), { recursive: true }).then(u => { unwatch = u; }).catch((e) => { console.error('watch modules failed:', e); });
        function onReload() { loadModules(); }
        window.addEventListener('knofoo:reload-modules', onReload);
        return () => { unwatch?.(); window.removeEventListener('knofoo:reload-modules', onReload); };
    });

    $effect(() => {
        if (!vault.path) return;
        let unwatch: (() => void) | undefined;
        watchImmediate(vault.path, debounce(loadVaultFiles, 80), { recursive: true }).then(u => { unwatch = u; }).catch(() => {});
        return () => unwatch?.();
    });

    // ── Context menu ──────────────────────────────────────────────────────────
    type MenuMode = 'idle' | 'rename';

    interface MenuState {
        x: number; y: number;
        section: Section;
        target: FileEntry | null;    // entry right-clicked (null = blank section area)
        targetDir: string;           // directory the target lives in (for rename/delete)
        mode: MenuMode;
        inputVal: string;
        error: string;
    }

    let menu    = $state<MenuState | null>(null);
    let menuEl  = $state<HTMLDivElement | undefined>(undefined);
    let inputEl = $state<HTMLInputElement | undefined>(undefined);

    function sectionRoot(section: Section): string {
        if (section === 'graph')  return graphsPath ?? '';
        if (section === 'module') return modulesPath ?? '';
        return vault.path ?? '';
    }

    // parentDir of an entry = the directory containing it (strip last segment from path)
    function parentDir(entry: FileEntry): string {
        const sep = entry.path.includes('/') ? '/' : '\\';
        return entry.path.substring(0, entry.path.lastIndexOf(sep));
    }

    function openMenu(e: MouseEvent, section: Section, target: FileEntry | null) {
        e.preventDefault();
        e.stopPropagation();
        const x = Math.min(e.clientX, window.innerWidth  - 200);
        const y = Math.min(e.clientY, window.innerHeight - 150);
        // targetDir: if clicking blank area → section root; if clicking entry → its parent dir
        const tDir = target ? parentDir(target) : sectionRoot(section);
        menu = { x, y, section, target, targetDir: tDir, mode: 'idle', inputVal: '', error: '' };
    }

    function closeMenu() { menu = null; }

    $effect(() => {
        if (!menu) return;
        function onPd(e: PointerEvent)  { if (!menuEl?.contains(e.target as Node)) closeMenu(); }
        function onKd(e: KeyboardEvent) { if (e.key === 'Escape') closeMenu(); }
        window.addEventListener('pointerdown', onPd);
        window.addEventListener('keydown', onKd);
        return () => { window.removeEventListener('pointerdown', onPd); window.removeEventListener('keydown', onKd); };
    });

    function stemOf(name: string) {
        const dot = name.lastIndexOf('.');
        return dot > 0 ? name.slice(0, dot) : name;
    }

function startRename(entry: FileEntry) {
        if (!menu) return;
        const inputVal = entry.name;
        menu = { ...menu, mode: 'rename', inputVal, error: '' };
        setTimeout(() => {
            inputEl?.focus();
            // select only the stem, not the extension
            const stemLen = entry.is_dir ? entry.name.length : stemOf(entry.name).length;
            inputEl?.setSelectionRange(0, stemLen);
        }, 0);
    }

    async function createJsonFile(dir: string, kind: string, section: Section) {
        try {
            const p = await uniquePath(dir, 'Untitled', 'json');
            await invoke('write_file', { path: p, content: JSON.stringify({ knofoo: '0.1.0', kind, nodes: [], edges: [] }, null, 2) });
            await reload(section);
            setFocus(section, p);
            if (section === 'graph' || section === 'module') graphStore.open(p, section);
        } catch (e) { console.error('createJsonFile:', e); }
        closeMenu();
    }

    async function createFileIn(dir: string, section: Section = 'vault') {
        try {
            const p = await uniquePath(dir, 'Untitled', 'md');
            await invoke('write_file', { path: p, content: '' });
            await reload(section);
            setFocus(section, p);
        } catch (e) { console.error('createFile:', e); }
        closeMenu();
    }

    async function createFolderIn(dir: string, section: Section = 'vault') {
        try {
            const p = await uniquePath(dir, 'Untitled', '');
            await invoke('mkdir_path', { path: p });
            await reload(section);
            setFocus(section, p);
        } catch (e) { console.error('createFolder:', e); }
        closeMenu();
    }

    function createGraph(dir?: string) { return createJsonFile(dir ?? graphsPath!, 'graph', 'graph'); }
    function createModule(dir?: string) { return createJsonFile(dir ?? modulesPath!, 'module', 'module'); }

    async function confirmRename() {
        if (!menu || !menu.target) return;
        const newName = menu.inputVal.trim();
        if (!newName || newName === menu.target.name) { closeMenu(); return; }
        try {
            await invoke('rename_path', { src: menu.target.path, dst: menu.targetDir + '/' + newName });
            await reload(menu.section);
            closeMenu();
        } catch (e) {
            menu = { ...menu, error: String(e) };
        }
    }

    async function deleteItem(entry: FileEntry, section: Section) {
        try {
            await invoke('remove_path', { path: entry.path });
            for (const tab of graphStore.tabs) {
                if (tab.path === entry.path || tab.path.startsWith(entry.path + '/') || tab.path.startsWith(entry.path + '\\')) {
                    graphStore.close(tab.id);
                }
            }
            await reload(section);
        } catch (e) { console.error('deleteItem:', e); }
        closeMenu();
    }

    function openItem(entry: FileEntry, section: 'graph' | 'module') {
        if (entry.is_dir) return;
        graphStore.open(entry.path, section);
    }

    function onInputKeydown(e: KeyboardEvent) {
        if (e.key === 'Enter')  { e.preventDefault(); confirmRename(); }
        if (e.key === 'Escape') { e.preventDefault(); closeMenu(); }
    }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="explorer" oncontextmenu={(e) => openMenu(e, 'vault', null)} role="region" aria-label="Explorer">
    {#if folderName}
        <div class="vault-header">
            <span class="vault-icon">⌂</span>
            <span class="vault-name" title={vault.path ?? ''}>{folderName}</span>
            <button class="vault-pick" title="Change vault folder"
                    onclick={(e) => { e.stopPropagation(); vault.pick(); }}>📁</button>
        </div>
    {:else}
        <div class="no-vault">
            <p>No vault selected</p>
            <button class="vault-pick-big" onclick={(e) => { e.stopPropagation(); vault.pick(); }}>
                📁 Choose vault folder…
            </button>
        </div>
    {/if}

    <!-- Graphs section -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="section-wrap" oncontextmenu={(e) => { e.stopPropagation(); openMenu(e, 'graph', null); }}>
        <div class="section-header" role="group">
            <span class="section-icon">◈</span>
            <span class="section-label">Graphs</span>
            <button class="add-btn" title="New graph" onclick={(e) => { e.stopPropagation(); createGraph(); }}>+</button>
        </div>
        {#if graphs.length === 0}
            <div class="empty-hint">right-click or + to create</div>
        {:else}
            <FileTree
                entries={graphs}
                section="graph"
                expandedPaths={graphExpanded}
                {focusPath}
                onToggle={(p, o) => onToggle('graph', p, o)}
                onContextMenu={(e, entry) => openMenu(e, 'graph', entry)}
                onOpen={(entry) => openItem(entry, 'graph')}
            />
        {/if}
    </div>

    <!-- Modules section -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="section-wrap" oncontextmenu={(e) => { e.stopPropagation(); openMenu(e, 'module', null); }}>
        <div class="section-header" role="group">
            <span class="section-icon">⊞</span>
            <span class="section-label">Modules</span>
            <button class="add-btn" title="New module" onclick={(e) => { e.stopPropagation(); createModule(); }}>+</button>
        </div>
        {#if modules.length === 0}
            <div class="empty-hint">right-click or + to create</div>
        {:else}
            <FileTree
                entries={modules}
                section="module"
                expandedPaths={moduleExpanded}
                {focusPath}
                onToggle={(p, o) => onToggle('module', p, o)}
                onContextMenu={(e, entry) => openMenu(e, 'module', entry)}
                onOpen={(entry) => openItem(entry, 'module')}
            />
        {/if}
    </div>

    <!-- Vault section -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="section-wrap section-wrap--grow" oncontextmenu={(e) => { e.stopPropagation(); openMenu(e, 'vault', null); }}>
        <div class="section-header section-header--vault" role="group">
            <span class="section-icon">⌂</span>
            <span class="section-label">Vault</span>
            <button class="add-btn" title="New file" onclick={(e) => { e.stopPropagation(); createFileIn(vault.path!); }}>+</button>
        </div>
        {#if vaultFiles.length === 0}
            <div class="empty-hint">right-click to create files</div>
        {:else}
            <FileTree
                entries={vaultFiles}
                section="vault"
                expandedPaths={vaultExpanded}
                {focusPath}
                onToggle={(p, o) => onToggle('vault', p, o)}
                onContextMenu={(e, entry) => openMenu(e, 'vault', entry)}
                onOpen={() => {}}
            />
        {/if}
    </div>

    <!-- Context menu -->
    {#if menu}
        <div
            bind:this={menuEl}
            class="context-menu"
            role="menu"
            tabindex="-1"
            style="left:{menu.x}px;top:{menu.y}px"
        >
            {#if menu.mode === 'idle'}
                {#if menu.target?.is_dir}
                    <button role="menuitem" onclick={() => createFileIn(menu!.target!.path, menu!.section)}>New File</button>
                    <button role="menuitem" onclick={() => createFolderIn(menu!.target!.path, menu!.section)}>New Folder</button>
                    {#if menu.section === 'graph'}
                        <button role="menuitem" onclick={() => createGraph(menu!.target!.path)}>New Graph</button>
                    {:else if menu.section === 'module'}
                        <button role="menuitem" onclick={() => createModule(menu!.target!.path)}>New Module</button>
                    {/if}
                    <div class="sep"></div>
                    <button role="menuitem" onclick={() => startRename(menu!.target!)}>Rename…</button>
                    <div class="sep"></div>
                    <button role="menuitem" class="danger" onclick={() => deleteItem(menu!.target!, menu!.section)}>Delete</button>
                {:else if menu.target !== null}
                    <button role="menuitem" onclick={() => startRename(menu!.target!)}>Rename…</button>
                    <div class="sep"></div>
                    <button role="menuitem" class="danger" onclick={() => deleteItem(menu!.target!, menu!.section)}>Delete</button>
                {:else}
                    {#if menu.section === 'graph'}
                        <button role="menuitem" onclick={() => createGraph()}>New Graph</button>
                        <button role="menuitem" onclick={() => createFolderIn(graphsPath!, 'graph')}>New Folder</button>
                    {:else if menu.section === 'module'}
                        <button role="menuitem" onclick={() => createModule()}>New Module</button>
                        <button role="menuitem" onclick={() => createFolderIn(modulesPath!, 'module')}>New Folder</button>
                    {:else}
                        <button role="menuitem" onclick={() => createFileIn(vault.path!, 'vault')}>New File</button>
                        <button role="menuitem" onclick={() => createFolderIn(vault.path!, 'vault')}>New Folder</button>
                    {/if}
                {/if}
            {:else}
                <div class="rename-row">
                    <input
                        bind:this={inputEl}
                        bind:value={menu.inputVal}
                        onkeydown={onInputKeydown}
                        class="rename-input"
                        spellcheck="false"
                    />
                </div>
                {#if menu.error}
                    <div class="menu-error">{menu.error}</div>
                {/if}
                <div class="menu-actions">
                    <button onclick={confirmRename}>Rename</button>
                    <button onclick={closeMenu}>Cancel</button>
                </div>
            {/if}
        </div>
    {/if}
</div>

<style>
    .explorer {
        display: flex;
        flex-direction: column;
        flex: 1;
        min-height: 0;
        overflow-y: auto;
        background: #0f0f17;
    }

    .vault-header {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 8px 12px;
        border-bottom: 1px solid rgba(255,255,255,0.06);
        background: rgba(255,255,255,0.02);
        flex-shrink: 0;
    }

    .vault-icon { font-size: 0.85rem; color: rgba(225,80,35,0.8); }

    .vault-name {
        flex: 1;
        font-size: 0.82rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        color: rgba(226,232,240,0.55);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .vault-pick {
        background: none;
        border: none;
        color: rgba(226,232,240,0.45);
        cursor: pointer;
        padding: 2px 6px;
        font-size: 0.85rem;
        border-radius: 3px;
        flex-shrink: 0;
    }
    .vault-pick:hover {
        background: rgba(255,255,255,0.08);
        color: #e2e8f0;
    }
    .no-vault {
        padding: 20px 16px;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 12px;
        text-align: center;
    }
    .no-vault p {
        margin: 0;
        font-size: 0.85rem;
        color: rgba(226,232,240,0.4);
    }
    .vault-pick-big {
        padding: 8px 16px;
        background: rgba(225,80,35,0.15);
        border: 1px solid rgba(225,80,35,0.45);
        border-radius: 5px;
        color: rgba(226,232,240,0.9);
        font-size: 0.85rem;
        font-family: monospace;
        cursor: pointer;
    }
    .vault-pick-big:hover {
        background: rgba(225,80,35,0.3);
    }

    .section-wrap {
        display: flex;
        flex-direction: column;
    }

    .section-header {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 8px 10px 4px 12px;
        flex-shrink: 0;
        cursor: default;
        user-select: none;
    }

    .section-header--vault {
        margin-top: 6px;
        border-top: 1px solid rgba(255,255,255,0.06);
        padding-top: 10px;
    }

    .section-wrap--grow { flex: 1; }

    .section-icon { font-size: 0.72rem; color: rgba(225,80,35,0.6); }

    .section-label {
        font-size: 0.75rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: rgba(226,232,240,0.35);
        flex: 1;
    }

    .add-btn {
        width: 18px;
        height: 18px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: none;
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 3px;
        color: rgba(226,232,240,0.4);
        font-size: 0.95rem;
        line-height: 1;
        cursor: pointer;
        padding: 0;
        transition: all 0.15s;
    }

    .add-btn:hover {
        background: rgba(225,80,35,0.2);
        border-color: rgba(225,80,35,0.5);
        color: #e15023;
    }

    .empty-hint {
        padding: 4px 12px 6px 28px;
        font-size: 0.78rem;
        font-family: monospace;
        color: rgba(226,232,240,0.15);
    }

    /* Context menu */
    .context-menu {
        position: fixed;
        z-index: 1000;
        background: #1a1a2e;
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 6px;
        padding: 4px;
        box-shadow: 0 8px 24px rgba(0,0,0,0.5);
        min-width: 160px;
    }

    .context-menu button[role="menuitem"] {
        display: block;
        width: 100%;
        padding: 6px 10px;
        background: none;
        border: none;
        border-radius: 4px;
        color: rgba(226,232,240,0.85);
        font-size: 0.92rem;
        text-align: left;
        cursor: pointer;
    }

    .context-menu button[role="menuitem"]:hover { background: rgba(225,80,35,0.15); color: #fff; }
    .context-menu button.danger { color: rgba(239,68,68,0.85); }
    .context-menu button.danger:hover { background: rgba(239,68,68,0.15); color: rgb(239,68,68); }

    .sep { height: 1px; background: rgba(255,255,255,0.07); margin: 3px 6px; }

    .rename-row {
        display: flex;
        align-items: center;
        gap: 4px;
        padding: 4px 6px;
    }

    .rename-input {
        flex: 1;
        background: rgba(255,255,255,0.06);
        border: 1px solid rgba(225,80,35,0.5);
        border-radius: 4px;
        color: rgba(226,232,240,0.9);
        font-size: 0.92rem;
        padding: 4px 6px;
        outline: none;
        min-width: 0;
    }

    .rename-input:focus { border-color: rgb(225,80,35); }

    .menu-error {
        font-size: 0.82rem;
        color: #e15023;
        padding: 2px 6px 4px;
    }

    .menu-actions {
        display: flex;
        gap: 4px;
        padding: 4px 6px;
    }

    .menu-actions button {
        flex: 1;
        padding: 4px 8px;
        background: none;
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 4px;
        color: rgba(226,232,240,0.7);
        font-size: 0.85rem;
        cursor: pointer;
    }

    .menu-actions button:first-child {
        background: rgba(225,80,35,0.2);
        border-color: rgba(225,80,35,0.4);
        color: rgba(226,232,240,0.9);
    }

    .menu-actions button:first-child:hover { background: rgba(225,80,35,0.35); }
    .menu-actions button:last-child:hover  { background: rgba(255,255,255,0.06); }
</style>
