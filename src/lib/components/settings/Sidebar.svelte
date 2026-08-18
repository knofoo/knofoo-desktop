<script lang="ts">
    import { page } from '$app/state';

    const items = [
        { label: 'General',    path: '/settings/general' },
        { label: 'Paths',      path: '/settings/paths' },
        { label: 'Editor',     path: '/settings/editor' },
        { label: 'Appearance', path: '/settings/appearance' },
        { label: 'Hotkeys',    path: '/settings/hotkeys' },
    ];
</script>

<aside class="sidebar">
    <h2 class="sidebar-title">Settings</h2>

    <nav class="nav">
        {#each items as item}
            <a
                href={item.path}
                class="nav-link"
                class:active={page.url.pathname === item.path}
            >
                {item.label}
            </a>
        {/each}
    </nav>
</aside>

<style>
    .sidebar {
        width: 200px;
        flex-shrink: 0;
        border-right: 1px solid rgba(255, 255, 255, 0.06);
        padding: 1.5rem 0.75rem;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    .sidebar-title {
        font-size: 0.85rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: rgba(226, 232, 240, 0.3);
        padding: 0 0.5rem;
        margin-bottom: 0.5rem;
    }

    .nav {
        display: flex;
        flex-direction: column;
        gap: 2px;
    }

    .nav-link {
        display: block;
        padding: 0.4rem 0.75rem;
        border-radius: 5px;
        font-size: 1rem;
        color: rgba(226, 232, 240, 0.6);
        text-decoration: none;
        transition: background 0.1s, color 0.1s;
    }

    .nav-link:hover {
        background: rgba(255, 255, 255, 0.05);
        color: rgba(226, 232, 240, 0.9);
    }

    .nav-link.active {
        background: rgba(225, 80, 35, 0.15);
        color: #e15023;
    }
</style>
