export type TabKind = 'graph' | 'module';

export interface Tab {
    id: string;
    kind: TabKind;
    name: string;
    path: string;
    dirty: boolean;
}

function createGraphStore() {
    let tabs   = $state<Tab[]>([]);
    let active = $state<string | null>(null);

    const activeTab = $derived(tabs.find(t => t.id === active) ?? null);

    const saveHandlers = new Map<string, () => Promise<void>>();

    function open(path: string, kind: TabKind) {
        const existing = tabs.find(t => t.path === path);
        if (existing) { active = existing.id; return; }
        const name = path.split(/[\\/]/).at(-1)?.replace(/\.json$/, '') ?? path;
        const tab: Tab = { id: crypto.randomUUID(), kind, name, path, dirty: false };
        tabs = [...tabs, tab];
        active = tab.id;
    }

    function close(id: string) {
        const idx = tabs.findIndex(t => t.id === id);
        tabs = tabs.filter(t => t.id !== id);
        saveHandlers.delete(id);
        if (active === id) {
            active = tabs[Math.min(idx, tabs.length - 1)]?.id ?? null;
        }
    }

    function rename(id: string, name: string) {
        tabs = tabs.map(t => t.id === id ? { ...t, name } : t);
    }

    function setDirty(id: string, dirty: boolean) {
        tabs = tabs.map(t => t.id === id ? { ...t, dirty } : t);
    }

    function setActive(id: string) {
        active = id;
    }

    function registerSave(id: string, fn: () => Promise<void>) {
        saveHandlers.set(id, fn);
    }

    async function save(id: string) {
        await saveHandlers.get(id)?.();
    }

    return {
        get tabs()      { return tabs; },
        get active()    { return active; },
        get activeTab() { return activeTab; },
        open,
        close,
        rename,
        setDirty,
        setActive,
        registerSave,
        save,
    };
}

export const graphStore = createGraphStore();
