<script lang="ts">
    import { onMount } from 'svelte';
    import { LGraph, LGraphCanvas, LiteGraph } from '@comfyorg/litegraph';
    import { invoke } from '@tauri-apps/api/core';
    import { registerContentNode } from './nodes/ContentNode';
    import { registerValidatorNode } from './nodes/ValidatorNode';
    import { registerModuleNode } from './nodes/ModuleNode';
    import { registerMachineNode, MachineNode } from '$lib/nodes/MachineNode';
    import NodeToolbar from './NodeToolbar.svelte';
    import CanvasMenu from './CanvasMenu.svelte';
    import MachineOverlay from '$lib/nodes/MachineOverlay.svelte';
    import type { LGraphNode } from '@comfyorg/litegraph';
    import { graphStore, type Tab } from '$lib/stores/graphStore.svelte';
    import { vault } from '$lib/stores/vaultStore.svelte';
    import { logStore } from '$lib/stores/logStore.svelte';

    // Map LiteGraph type string → knofoo type string
    const LG_TO_KNOFOO: Record<string, string> = {
        'knofoo/content':   'markdown',
        'knofoo/validator': 'qa',
        'knofoo/module':    'module',
        'knofoo/machine':   'machine',
    };
    const KNOFOO_TO_LG: Record<string, string> = {
        'markdown': 'knofoo/content',
        'qa':       'knofoo/validator',
        'module':   'knofoo/module',
        'machine':  'knofoo/machine',
    };

    let existingMeta: Record<string, any> = {};
    // Serialized snapshot of the last saved (or loaded) state. Dirty = current
    // serialization differs from this. We can't key dirty off graph.change()
    // because LiteGraph fires it on every key event (incl. keyup), which would
    // re-mark the graph modified right after a save.
    let savedSig = '';

    function toKnofoo(g: LGraph): object {
        const nodes = (g as any)._nodes as LGraphNode[];
        const links = (g as any)._links as Map<number, any> | Record<number, any>;

        const knofooNodes = (nodes ?? []).map((n: LGraphNode) => ({
            id:       `node_${n.id}`,
            type:     LG_TO_KNOFOO[(n as any).type ?? ''] ?? (n as any).type ?? 'markdown',
            position: { x: n.pos[0], y: n.pos[1] },
            size:     { w: n.size[0], h: n.size[1] },
            meta:     {
                title:   n.title ?? '',
                tags:    [],
                locked:  !!(n as any).flags?.locked,
                visible: !((n as any).flags?.collapsed),
            },
            data:        { ...(n.properties ?? {}) },
            connections: { requires: [], unlocks: [] },
            proof:       { strategy: 'none', weight: 1.0 },
        }));

        // Build an id→index map for edges
        const nodeIdMap = new Map<string | number, string>();
        (nodes ?? []).forEach((n: LGraphNode) => nodeIdMap.set(n.id as any, `node_${n.id}`));

        let linkArr: any[] = [];
        if (links instanceof Map) {
            linkArr = [...links.values()];
        } else if (links && typeof links === 'object') {
            linkArr = Object.values(links);
        }

        const knofooEdges = linkArr.map((l: any) => ({
            id:        `edge_${l.id}`,
            from:      nodeIdMap.get(l.origin_id)   ?? `node_${l.origin_id}`,
            to:        nodeIdMap.get(l.target_id)   ?? `node_${l.target_id}`,
            from_slot: l.origin_slot  ?? 0,
            to_slot:   l.target_slot  ?? 0,
        }));

        return {
            knofoo:  '0.1.0',
            id:      existingMeta.id      ?? '',
            meta: {
                title:       existingMeta.title       ?? '',
                description: existingMeta.description ?? '',
                authors:     existingMeta.authors     ?? [],
                tags:        existingMeta.tags        ?? [],
                license:     existingMeta.license     ?? 'CC-BY-SA-4.0',
                lang:        existingMeta.lang        ?? 'en',
            },
            graph: { nodes: knofooNodes, edges: knofooEdges },
            version: { commit: '', parent: null },
        };
    }

    function fromKnofoo(data: any, g: LGraph) {
        existingMeta = { ...(data?.meta ?? {}), id: data?.id ?? '' };
        g.clear();
        const nodeMap = new Map<string, LGraphNode>();

        for (const n of (data?.graph?.nodes ?? [])) {
            const lgType = KNOFOO_TO_LG[n.type] ?? n.type;
            const node = LiteGraph.createNode(lgType);
            if (!node) continue;
            node.pos  = [n.position?.x ?? 0, n.position?.y ?? 0];
            if (n.size) node.size = [n.size.w, n.size.h];
            if (n.meta?.title) {
                node.title = n.meta.title;
                (node.properties as any).title = n.meta.title;
            }
            if (n.data) Object.assign(node.properties, n.data);
            // Sync LiteGraph color props from properties
            if ((node.properties as any).color)   (node as any).color   = (node.properties as any).color;
            if ((node.properties as any).bgcolor)  (node as any).bgcolor = (node.properties as any).bgcolor;
            g.add(node);
            nodeMap.set(n.id, node);
        }

        for (const e of (data?.graph?.edges ?? [])) {
            const fromNode = nodeMap.get(e.from);
            const toNode   = nodeMap.get(e.to);
            if (fromNode && toNode) {
                fromNode.connect(e.from_slot ?? 0, toNode, e.to_slot ?? 0);
            }
        }
    }

    interface Props { tab: Tab; onModuleNodeDblClick?: (path: string) => void; }
    let { tab: _tab, onModuleNodeDblClick }: Props = $props();

    registerContentNode();
    registerValidatorNode();
    registerModuleNode();
    registerMachineNode();

    let wrapEl:   HTMLDivElement    = $state(undefined!);
    let canvasEl: HTMLCanvasElement = $state(undefined!);

    // Machine overlays: Svelte state only for node identity (add/remove).
    // Positions are updated imperatively via setRect to avoid per-frame reactivity.
    let machineNodes = $state<MachineNode[]>([]);
    // svelte-ignore non_reactive_update
    const overlaySetRect = new Map<string, (x: number, y: number, w: number, h: number, visible: boolean) => void>();

    // svelte-ignore non_reactive_update
    let lastSyncKey = '';

    function syncMachineOverlays() {
        if (!lgCanvas || !graph || !canvasEl) return;
        const nodes = (graph as any)._nodes as LGraphNode[];
        if (!nodes) return;

        const scale = lgCanvas.ds.scale;
        const ox    = lgCanvas.ds.offset[0];
        const oy    = lgCanvas.ds.offset[1];

        const cur = nodes.filter(n => (n as any).type === 'knofoo/machine') as unknown as MachineNode[];

        // Skip when nothing visible has changed: viewport, node count, or each
        // machine node's pos/size are all stable. Keeps the RAF loop idle
        // when the user isn't panning/zooming/dragging.
        let posSig = '';
        for (const mn of cur) {
            const n = mn as unknown as LGraphNode;
            posSig += `|${mn.vmId}:${n.pos[0]},${n.pos[1]},${n.size[0]},${n.size[1]}`;
        }
        const syncKey = `${scale}|${ox}|${oy}|${canvasEl.width}x${canvasEl.height}${posSig}`;
        if (syncKey === lastSyncKey) return;
        lastSyncKey = syncKey;

        // Detect add/remove (only triggers Svelte re-render on change)
        const curIds = new Set(cur.map(mn => mn.vmId));
        const prevIds = new Set(machineNodes.map(mn => mn.vmId));
        if (curIds.size !== prevIds.size || [...curIds].some(id => !prevIds.has(id))) {
            machineNodes = cur;
        }

        const rect = canvasEl.getBoundingClientRect();
        for (const mn of cur) {
            const n = mn as unknown as LGraphNode;
            const x = (n.pos[0] + ox) * scale;
            const y = (n.pos[1] + oy) * scale;
            const w = n.size[0] * scale;
            const h = n.size[1] * scale;
            const visible = x + w > 0 && y + h > 0 && x < rect.width && y < rect.height;
            overlaySetRect.get(mn.vmId)?.(x, y, w, h, visible);
        }
    }

    let canvasMenu = $state<{ x: number; y: number; gx: number; gy: number } | null>(null);
    let deleteConfirm = $state<{ nodes: LGraphNode[]; machineCount: number } | null>(null);
    let toolbar    = $state<{ node: LGraphNode; x: number; y: number } | null>(null);
    let showJson   = $state(false);
    let jsonText   = $state('');

    // svelte-ignore non_reactive_update
    let graph:    LGraph;
    // svelte-ignore non_reactive_update
    let lgCanvas: LGraphCanvas;

    function screenToGraph(sx: number, sy: number): [number, number] {
        const rect = canvasEl.getBoundingClientRect();
        const cx = sx - rect.left;
        const cy = sy - rect.top;
        // Use LiteGraph's authoritative conversion so placement always
        // matches what the user sees, regardless of pan/zoom state.
        const out: [number, number] = [0, 0];
        (lgCanvas.ds as any).convertCanvasToOffset([cx, cy], out);
        return out;
    }

    const TOOLBAR_W = 220;

    let toolbarEl = $state<HTMLDivElement | undefined>(undefined);

    function toolbarPos(node: LGraphNode) {
        const scale = lgCanvas.ds.scale;
        const titleH = ((LiteGraph as any).NODE_TITLE_HEIGHT ?? 30) * scale;
        const sw = node.size[0] * scale;
        const sx = (node.pos[0] + lgCanvas.ds.offset[0]) * scale;
        const sy = (node.pos[1] + lgCanvas.ds.offset[1]) * scale - titleH;
        const h = toolbarEl?.offsetHeight ?? 210;
        return {
            x: sx + sw / 2 - TOOLBAR_W / 2,
            y: sy - h - 12,
        };
    }

    function updateToolbar() {
        const selected = [...lgCanvas.selectedItems].filter(i => (i as any).inputs !== undefined) as LGraphNode[];
        const node = selected.length === 1 ? selected[0] : null;

        if (!node) {
            if (toolbar) toolbar = null;
            return;
        }

        const pos = toolbarPos(node);
        if (toolbar?.node === node && pos.x === toolbar.x && pos.y === toolbar.y) return;
        toolbar = { node, x: pos.x, y: pos.y };
    }

    function addNode(type: string, gx: number, gy: number) {
        if (type === 'knofoo/module') {
            createAndAddModuleNode(gx, gy);
            return;
        }
        const node = LiteGraph.createNode(type);
        if (!node) return;
        // gx,gy is the top-left of the menu (= the click point in graph
        // coordinates). LiteGraph node.pos is also top-left of the node body
        // (excluding title bar, which renders above), so place directly.
        node.pos = [gx, gy];
        graph.add(node);
        graph.change();
    }

    async function createAndAddModuleNode(gx: number, gy: number) {
        const modulesDir = vault.resolvePath(vault.config.paths.modules);
        try {
            const path = await invoke<string>('create_module', { vaultPath: modulesDir, name: null });
            addModuleNode(path, path.split(/[\\/]/).at(-1)?.replace(/\.json$/, '') ?? 'Untitled', gx, gy);
            window.dispatchEvent(new CustomEvent('knofoo:reload-modules'));
        } catch (e) { console.error('create module file failed:', e); }
    }

    function addModuleNode(path: string, name: string, gx: number, gy: number) {
        const node = LiteGraph.createNode('knofoo/module');
        if (!node) return;
        node.pos = [gx, gy];
        node.properties.path  = path;
        node.properties.title = name;
        (node as any).title   = name;
        graph.add(node);
        graph.change();
    }

    function onDragOver(e: DragEvent) {
        if (e.dataTransfer?.types.includes('application/knofoo-file')) {
            e.preventDefault();
            e.dataTransfer.dropEffect = 'copy';
        }
    }

    function onDrop(e: DragEvent) {
        const raw = e.dataTransfer?.getData('application/knofoo-file');
        if (!raw || !lgCanvas) return;
        e.preventDefault();
        try {
            const { path, name } = JSON.parse(raw);
            const rect = canvasEl.getBoundingClientRect();
            const gx = (e.clientX - rect.left) / lgCanvas.ds.scale - lgCanvas.ds.offset[0];
            const gy = (e.clientY - rect.top)  / lgCanvas.ds.scale - lgCanvas.ds.offset[1];
            addModuleNode(path, name, gx, gy);
        } catch { /* bad payload */ }
    }

    function deleteNode(node: LGraphNode) {
        graph.remove(node);
        toolbar = null;
        graph.change();
    }

    // Grid is cached to an offscreen canvas — re-rendered only when pan/zoom
    // or size changes, then blitted into LiteGraph's bgcanvas (drawBackCanvas).
    // The foreground blits bgcanvas each frame, so the dot pass runs at most
    // once per frame (and only when the view actually changed).
    // svelte-ignore non_reactive_update
    let gridCache: HTMLCanvasElement | null = null;
    // svelte-ignore non_reactive_update
    let gridKey   = '';

    function drawGrid(ctx: CanvasRenderingContext2D, w: number, h: number) {
        const scale = lgCanvas.ds.scale;
        const ox    = lgCanvas.ds.offset[0] * scale;
        const oy    = lgCanvas.ds.offset[1] * scale;
        // Cache key: rebuild only when these change.
        const key   = `${w}x${h}|${scale.toFixed(3)}|${ox.toFixed(1)}|${oy.toFixed(1)}`;
        if (key !== gridKey || !gridCache || gridCache.width !== w || gridCache.height !== h) {
            if (!gridCache) gridCache = document.createElement('canvas');
            if (gridCache.width !== w || gridCache.height !== h) {
                gridCache.width = w;
                gridCache.height = h;
            }
            const gctx = gridCache.getContext('2d')!;
            paintGridInto(gctx, w, h, scale, ox, oy);
            gridKey = key;
        }
        ctx.drawImage(gridCache, 0, 0);
    }

    function paintGridInto(ctx: CanvasRenderingContext2D, w: number, h: number, scale: number, ox: number, oy: number) {
        const spacing = Math.max(15, 20 * scale);
        const startX = ((ox % spacing) + spacing) % spacing;
        const startY = ((oy % spacing) + spacing) % spacing;

        ctx.fillStyle = '#0d0d12';
        ctx.fillRect(0, 0, w, h);

        const dotR = Math.max(0.8, 1.2 * Math.min(scale, 1.5));
        ctx.fillStyle = 'rgba(255,255,255,0.13)';

        for (let x = startX; x < w; x += spacing) {
            for (let y = startY; y < h; y += spacing) {
                ctx.beginPath();
                ctx.arc(x, y, dotR, 0, Math.PI * 2);
                ctx.fill();
            }
        }
    }

    async function loadGraph() {
        try {
            const raw = await invoke<string>('read_file', { path: _tab.path });
            try {
                const data = JSON.parse(raw);
                if (data?.knofoo) {
                    fromKnofoo(data, graph);
                } else {
                    graph.configure(data);
                }
            } catch (e) { console.error('load parse failed:', e); }
        } catch (e) { console.error('load read failed:', e, _tab.path); }
    }

    async function saveGraph() {
        if (!graph) return;
        try {
            const payload = JSON.stringify(toKnofoo(graph), null, 2);
            await invoke('write_file', { path: _tab.path, content: payload });
            savedSig = payload;
            graphStore.setDirty(_tab.id, false);
        } catch (e) {
            console.error('save failed:', e, _tab.path);
        }
    }

    onMount(() => {
        graphStore.registerSave(_tab.id, saveGraph);
        canvasEl.width  = wrapEl.clientWidth;
        canvasEl.height = wrapEl.clientHeight;

        graph    = new LGraph();
        lgCanvas = new LGraphCanvas(canvasEl, graph);

        lgCanvas.background_image            = '';
        lgCanvas.render_shadows              = false;
        (lgCanvas as any).render_canvas_border       = false;
        (lgCanvas as any).render_connections_border  = false;
        lgCanvas.clear_background            = false;

        // Override LiteGraph's background draw with our dot grid.
        // The grid is painted ONLY here (into the offscreen bgcanvas). The
        // front canvas blits bgcanvas over itself after clearing (see
        // LGraphCanvas.drawFrontCanvas), so the grid reaches the foreground
        // with a single GPU-accelerated copy — no second dot pass needed, and
        // the blit itself erases dragged-node ghost trails.
        (lgCanvas as any).drawBackCanvas = function() {
            const bg = (this as any).bgcanvas;
            const ctx = bg?.getContext('2d') ?? (canvasEl ? canvasEl.getContext('2d') : null);
            if (!ctx) return;
            drawGrid(ctx, canvasEl.width, canvasEl.height);
        };

        // Hide LiteGraph default UI noise (T: value, info overlay)
        (lgCanvas as any).show_info = false;
        lgCanvas.allow_searchbox = false;

        // Snap to grid — reactive to settings changes
        $effect(() => {
            const snap = vault.config.editor?.snapToGrid ?? false;
            const size = vault.config.editor?.gridSize   ?? 10;
            (lgCanvas as any).snap_to_grid = snap ? size : 0;
        });

        let loading = true;
        // Lazy JSON refresh — only when the panel is actually open. Throttled
        // by a microtask flag so a burst of changes only repaints once.
        let jsonRefreshPending = false;
        function scheduleJsonRefresh() {
            if (!showJson || jsonRefreshPending) return;
            jsonRefreshPending = true;
            queueMicrotask(() => {
                jsonRefreshPending = false;
                if (showJson) jsonText = JSON.stringify(toKnofoo(graph), null, 2);
            });
        }
        // Dirty = serialization differs from the last saved/loaded snapshot.
        // Debounced so a burst of change() calls (e.g. during a node drag, or
        // LiteGraph's per-keystroke change()) only serializes once, keeping this
        // cheap even on large graphs. A keypress that changes nothing leaves the
        // signature equal → not dirty, so the save indicator stops flickering.
        let dirtyTimer: ReturnType<typeof setTimeout> | null = null;
        function scheduleDirtyCheck() {
            if (loading) return;
            scheduleJsonRefresh();
            if (dirtyTimer !== null) clearTimeout(dirtyTimer);
            dirtyTimer = setTimeout(() => {
                dirtyTimer = null;
                const sig = JSON.stringify(toKnofoo(graph), null, 2);
                graphStore.setDirty(_tab.id, sig !== savedSig);
            }, 150);
        }
        graph.onNodeAdded        = () => scheduleDirtyCheck();
        graph.onNodeRemoved      = () => scheduleDirtyCheck();
        graph.onConnectionChange = () => scheduleDirtyCheck();
        const origChange = graph.change.bind(graph);
        graph.change = () => {
            origChange();
            scheduleDirtyCheck();
        };

        loadGraph().then(() => {
            loading = false;
            savedSig = JSON.stringify(toKnofoo(graph), null, 2);
            if (showJson) jsonText = savedSig;
            lgCanvas.setDirty(true, true);
            lgCanvas.draw(true, true);
        });

        // Keyboard shortcuts
        function onKeydown(e: KeyboardEvent) {
            // Every tab's canvas is mounted at once (inactive ones are just
            // visibility:hidden), so each adds its own window keydown listener.
            // Without this guard Ctrl+S would fire on all of them and save every
            // open graph. Only the active tab handles global shortcuts.
            if (graphStore.active !== _tab.id) return;

            // toLowerCase: Caps Lock / some webviews report 'S', and `=== 's'`
            // would then miss — leaving the browser's own Ctrl+S to fire.
            if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
                e.preventDefault();
                saveGraph();
                return;
            }
            // Skip when user is typing in an input/textarea/contenteditable,
            // or focus is inside a VM display overlay (the VM owns the keys).
            const t = e.target as HTMLElement | null;
            if (t) {
                const tag = t.tagName;
                if (tag === 'INPUT' || tag === 'TEXTAREA' || t.isContentEditable) return;
                if (t.closest('.machine-overlay, .fullscreen-modal')) return;
            }
            // Delete only on Delete key, never Backspace (Backspace is the
            // back-navigation / text-edit key on most keyboards and should
            // never destroy nodes). Also require canvas to be the active area.
            if (e.key === 'Delete') {
                e.preventDefault();
                const selected = [...lgCanvas.selectedItems].filter(i => (i as any).inputs !== undefined) as LGraphNode[];
                if (selected.length === 0) return;
                const machineCount = selected.filter(n => (n as any).type === 'knofoo/machine').length;
                deleteConfirm = { nodes: selected, machineCount };
            }
        }
        window.addEventListener('keydown', onKeydown);

        // When a module file is renamed from the panel, fix up any module
        // nodes in this graph that referenced the old path.
        function onModuleRenamed(ev: Event) {
            const { oldPath, newPath } = (ev as CustomEvent).detail as { oldPath: string; newPath: string };
            const nodes = (graph as any)._nodes as LGraphNode[] | undefined;
            if (!nodes) return;
            let changed = false;
            for (const n of nodes) {
                const props = (n as any).properties;
                if (props?.path === oldPath) {
                    props.path = newPath;
                    changed = true;
                }
            }
            if (changed) {
                lgCanvas.setDirty(true, true);
                graph.change();
                // Persist so the graph's stored path stays consistent with disk.
                saveGraph();
            }
        }
        window.addEventListener('knofoo:module-renamed', onModuleRenamed);

        function onModuleMeta(ev: Event) {
            const { path: p, title: t, description: d } = (ev as CustomEvent).detail as { path: string; title: string; description: string };
            const nodes = (graph as any)._nodes as LGraphNode[] | undefined;
            if (!nodes) return;
            let changed = false;
            for (const n of nodes) {
                const props = (n as any).properties;
                if (props?.path === p) {
                    props.title       = t;
                    props.description = d;
                    // LiteGraph draws node.title in the title bar — sync from
                    // the user-set title. Empty string is allowed.
                    (n as any).title  = t || '';
                    changed = true;
                }
            }
            if (changed) {
                lgCanvas.setDirty(true, true);
                graph.change();
                saveGraph();
            }
        }
        window.addEventListener('knofoo:module-meta-updated', onModuleMeta);

        canvasEl.addEventListener('pointerdown', () => {
            if (canvasMenu) canvasMenu = null;
        }, { capture: true });

        canvasEl.addEventListener('contextmenu', (e: MouseEvent) => {
            const cRect = canvasEl.getBoundingClientRect();
            const ox = e.clientX - cRect.left;
            const oy = e.clientY - cRect.top;
            const gx = ox / lgCanvas.ds.scale - lgCanvas.ds.offset[0];
            const gy = oy / lgCanvas.ds.scale - lgCanvas.ds.offset[1];
            const node = graph.getNodeOnPos(gx, gy);
            if (node) return;
            e.preventDefault();
            e.stopPropagation();
            const [sgx, sgy] = screenToGraph(e.clientX, e.clientY);
            const rect = wrapEl.getBoundingClientRect();
            canvasMenu = { x: e.clientX - rect.left, y: e.clientY - rect.top, gx: sgx, gy: sgy };
        });

        lgCanvas.onNodeDblClicked = (node: LGraphNode) => {
            const type = (node as any).type;
            if (type === 'knofoo/module') {
                const path = (node as any).properties?.path;
                if (path) onModuleNodeDblClick?.(path);
            }
        };

        // Force redraw when this tab becomes active (slot goes from hidden → visible)
        $effect(() => {
            if (graphStore.active === _tab.id && lgCanvas) {
                lgCanvas.setDirty(true, true);
                lgCanvas.draw(true, true);
            }
        });

        // Update toolbar + machine overlays on pan/zoom
        let rafId: number;
        const trackTransform = () => {
            updateToolbar();
            syncMachineOverlays();
            rafId = requestAnimationFrame(trackTransform);
        };
        rafId = requestAnimationFrame(trackTransform);

        graph.start();
        lgCanvas.setDirty(true, true);

        let pendingResizeRaf: number | null = null;
        const ro = new ResizeObserver(() => {
            const w = wrapEl.clientWidth;
            const h = wrapEl.clientHeight;
            if (canvasEl.width === w && canvasEl.height === h) return;
            canvasEl.width  = w;
            canvasEl.height = h;
            lgCanvas.resize(w, h);
            lgCanvas.setDirty(true, true);
            // Coalesce redraws — during user-driven resize (divider drag,
            // window resize) the observer fires many times per frame; a single
            // rAF-aligned draw is enough.
            if (pendingResizeRaf === null) {
                pendingResizeRaf = requestAnimationFrame(() => {
                    pendingResizeRaf = null;
                    lgCanvas.draw(true, true);
                });
            }
        });
        ro.observe(wrapEl);

        return () => {
            cancelAnimationFrame(rafId);
            if (dirtyTimer !== null) clearTimeout(dirtyTimer);
            ro.disconnect();
            graph.stop();
            window.removeEventListener('keydown', onKeydown);
            window.removeEventListener('knofoo:module-renamed', onModuleRenamed);
            window.removeEventListener('knofoo:module-meta-updated', onModuleMeta);
        };
    });
</script>

<div class="canvas-wrap" bind:this={wrapEl} ondragover={onDragOver} ondrop={onDrop} role="application" aria-label="Graph canvas">
    <canvas bind:this={canvasEl} class="lg-canvas"></canvas>

    <button
        class="json-toggle"
        class:active={showJson}
        title="View JSON"
        onclick={() => { if (graph) { jsonText = JSON.stringify(toKnofoo(graph), null, 2); showJson = !showJson; } }}
    >&#123; &#125;</button>

    {#if showJson}
        <div class="json-panel">
            <div class="json-panel-header">
                <span>JSON</span>
                <button onclick={() => { showJson = false; }}>×</button>
            </div>
            <pre class="json-content">{jsonText}</pre>
        </div>
    {/if}

    <div class="overlay">
        {#each machineNodes as mn (mn.vmId)}
            <MachineOverlay
                node={mn}
                onregister={(fn) => overlaySetRect.set(mn.vmId, fn)}
                onunregister={() => overlaySetRect.delete(mn.vmId)}
            />
        {/each}

        {#if canvasMenu}
            <CanvasMenu
                x={canvasMenu.x}
                y={canvasMenu.y}
                onaddcontent={() => addNode('knofoo/content', canvasMenu!.gx, canvasMenu!.gy)}
                onaddvalidator={() => addNode('knofoo/validator', canvasMenu!.gx, canvasMenu!.gy)}
                onaddmodule={() => addNode('knofoo/module', canvasMenu!.gx, canvasMenu!.gy)}
                onaddmachine={() => addNode('knofoo/machine', canvasMenu!.gx, canvasMenu!.gy)}
                onclose={() => { canvasMenu = null; }}
            />
        {/if}

        {#if toolbar}
            <div bind:this={toolbarEl} style="position:absolute;left:{toolbar.x}px;top:{toolbar.y}px;pointer-events:auto">
                <NodeToolbar
                    node={toolbar.node}
                    {lgCanvas}
                    onclose={() => { toolbar = null; }}
                    ondelete={() => deleteNode(toolbar!.node)}
                />
            </div>
        {/if}

        {#if deleteConfirm}
            <div class="modal-backdrop" role="presentation"
                 onclick={() => { deleteConfirm = null; }}
                 onkeydown={(e) => { if (e.key === 'Escape') deleteConfirm = null; }}>
                <div class="modal" role="dialog" aria-modal="true"
                     onclick={(e) => e.stopPropagation()}
                     onkeydown={(e) => e.stopPropagation()}>
                    <div class="modal-title">Delete {deleteConfirm.nodes.length} node{deleteConfirm.nodes.length === 1 ? '' : 's'}?</div>
                    <div class="modal-body">
                        {#if deleteConfirm.machineCount > 0}
                            <p class="warn">⚠ {deleteConfirm.machineCount} Machine (VM) node{deleteConfirm.machineCount === 1 ? '' : 's'} selected.</p>
                            <p>Deleting will stop the VM and discard its disk state. This cannot be undone.</p>
                        {:else}
                            <p>This cannot be undone.</p>
                        {/if}
                    </div>
                    <div class="modal-actions">
                        <button class="btn" onclick={() => { deleteConfirm = null; }}>Cancel</button>
                        <button class="btn danger" onclick={() => {
                            const c = deleteConfirm;
                            if (!c) return;
                            for (const n of c.nodes) {
                                if ((n as any).type === 'knofoo/machine') {
                                    try { (n as unknown as { stop?: () => Promise<void> }).stop?.(); } catch { /* noop */ }
                                }
                                graph.remove(n);
                            }
                            toolbar = null;
                            graph.change();
                            deleteConfirm = null;
                        }}>Delete</button>
                    </div>
                </div>
            </div>
        {/if}
    </div>
</div>

<style>
    .modal-backdrop {
        position: absolute; inset: 0; z-index: 100;
        background: rgba(0,0,0,0.55);
        display: flex; align-items: center; justify-content: center;
        pointer-events: auto;
    }
    .modal {
        background: #14141c;
        border: 1px solid rgba(255,255,255,0.12);
        border-radius: 8px;
        min-width: 360px; max-width: 480px;
        font-family: monospace;
        color: #e2e8f0;
        box-shadow: 0 12px 40px rgba(0,0,0,0.6);
    }
    .modal-title {
        padding: 12px 16px;
        font-size: 13px;
        border-bottom: 1px solid rgba(255,255,255,0.08);
    }
    .modal-body {
        padding: 12px 16px;
        font-size: 12px;
        color: rgba(226,232,240,0.75);
        line-height: 1.5;
    }
    .modal-body p { margin: 4px 0; }
    .modal-body .warn { color: #fbbf24; font-weight: 600; }
    .modal-actions {
        display: flex; justify-content: flex-end; gap: 8px;
        padding: 10px 16px;
        border-top: 1px solid rgba(255,255,255,0.06);
    }
    .modal-actions .btn {
        padding: 5px 14px;
        background: rgba(255,255,255,0.05);
        border: 1px solid rgba(255,255,255,0.12);
        border-radius: 4px;
        color: rgba(226,232,240,0.85);
        font-family: monospace; font-size: 11px;
        cursor: pointer;
    }
    .modal-actions .btn:hover { background: rgba(255,255,255,0.1); color: #e2e8f0; }
    .modal-actions .btn.danger {
        color: #f472b6;
        border-color: rgba(244,114,182,0.35);
    }
    .modal-actions .btn.danger:hover { background: rgba(244,114,182,0.12); }

    .canvas-wrap {
        position: relative;
        flex: 1;
        width: 100%;
        height: 100%;
        min-height: 0;
        background: #0d0d12;
    }

    .lg-canvas {
        position: absolute;
        inset: 0;
        display: block;
    }

    .overlay {
        position: absolute;
        inset: 0;
        pointer-events: none;
        z-index: 10;
        overflow: visible;
    }

    .json-toggle {
        position: absolute;
        bottom: 10px;
        right: 10px;
        z-index: 20;
        background: rgba(22,22,42,0.85);
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 5px;
        color: rgba(226,232,240,0.5);
        font-family: monospace;
        font-size: 0.78rem;
        padding: 4px 8px;
        cursor: pointer;
        transition: all 0.15s;
    }
    .json-toggle:hover, .json-toggle.active {
        background: rgba(225,80,35,0.15);
        border-color: rgba(225,80,35,0.4);
        color: rgba(226,232,240,0.9);
    }

    .json-panel {
        position: absolute;
        bottom: 44px;
        right: 10px;
        z-index: 20;
        width: 420px;
        max-height: 60vh;
        background: #0f0f1a;
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 8px;
        box-shadow: 0 8px 32px rgba(0,0,0,0.6);
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    .json-panel-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 6px 10px;
        border-bottom: 1px solid rgba(255,255,255,0.07);
        font-size: 0.75rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        color: rgba(226,232,240,0.35);
        flex-shrink: 0;
    }
    .json-panel-header button {
        background: none;
        border: none;
        color: rgba(226,232,240,0.4);
        cursor: pointer;
        font-size: 1rem;
        line-height: 1;
        padding: 0 2px;
    }
    .json-panel-header button:hover { color: rgba(226,232,240,0.9); }

    .json-content {
        margin: 0;
        padding: 10px;
        font-family: monospace;
        font-size: 0.78rem;
        color: rgba(226,232,240,0.7);
        overflow: auto;
        white-space: pre;
        flex: 1;
        min-height: 0;
    }
</style>
