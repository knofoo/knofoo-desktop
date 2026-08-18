<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';

  let {
    onnew = () => {},
    onopen = () => {},
    onsave = () => {},
    onexport = () => {},
    onfitView = () => {},
    oncenter = () => {},
    onminimap = () => {},
    onaccount = () => {},
    ontoggleExplorer = () => {},
    ontoggleGraph = () => {},
    ontoggleSettings = () => {},
    showExplorer = true,
    showGraph = true,
    showSettings = true,
    workspaceControls = true,
  }: {
    onnew?: () => void;
    onopen?: () => void;
    onsave?: () => void;
    onexport?: () => void;
    onfitView?: () => void;
    oncenter?: () => void;
    onminimap?: () => void;
    onaccount?: () => void;
    ontoggleExplorer?: () => void;
    ontoggleGraph?: () => void;
    ontoggleSettings?: () => void;
    showExplorer?: boolean;
    showGraph?: boolean;
    showSettings?: boolean;
    workspaceControls?: boolean;
  } = $props();

  let fileOpen = $state(false);
  let viewOpen = $state(false);

  function close() {
    fileOpen = false;
    viewOpen = false;
  }

  $effect(() => {
    window.addEventListener('click', close);
    return () => window.removeEventListener('click', close);
  });
</script>

<header class="topbar">
  <div class="logo">
    <svg width="18" height="18" viewBox="0 0 22 22" fill="none">
      <rect width="22" height="22" rx="5" fill="#e2e8f0"/>
      <text x="11" y="15.5" text-anchor="middle" font-size="11" font-weight="700"
        font-family="monospace" fill="#0f0f17">K</text>
    </svg>
    <span class="logo-name">knofoo</span>
  </div>

  <nav class="menu">
    <!-- File -->
    <div class="menu-item" class:open={fileOpen}>
      <button onclick={(e) => { e.stopPropagation(); fileOpen = !fileOpen; viewOpen = false; }}>
        File
      </button>
      {#if fileOpen}
        <div role="menu" tabindex="-1" class="dropdown" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
          <button role="menuitem" onclick={() => { onnew(); close(); }}>New graph</button>
          <button role="menuitem" onclick={() => { onopen(); close(); }}>Open…</button>
          <div role="presentation" class="sep"></div>
          <button role="menuitem" onclick={() => { onsave(); close(); }}>Save</button>
          <button role="menuitem" onclick={() => { onexport(); close(); }}>Export as JSON</button>
        </div>
      {/if}
    </div>

    <!-- View -->
    <div class="menu-item" class:open={viewOpen}>
      <button onclick={(e) => { e.stopPropagation(); viewOpen = !viewOpen; fileOpen = false; }}>
        View
      </button>
      {#if viewOpen}
        <div role="menu" tabindex="-1" class="dropdown" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
          {#if workspaceControls}
            <div role="presentation" class="dropdown-label">Panels</div>
            <button role="menuitem" onclick={() => { ontoggleExplorer(); close(); }}>
              <span class="check">{showExplorer ? '✓' : ''}</span>
              Explorer
            </button>
            <button role="menuitem" onclick={() => { ontoggleGraph(); close(); }}>
              <span class="check">{showGraph ? '✓' : ''}</span>
              Graph
            </button>
            <button role="menuitem" onclick={() => { ontoggleSettings(); close(); }}>
              <span class="check">{showSettings ? '✓' : ''}</span>
              Settings
            </button>
            <div role="presentation" class="sep"></div>
            <div role="presentation" class="dropdown-label">Graph</div>
            <button role="menuitem" onclick={() => { onfitView(); close(); }}>Fit to view</button>
            <button role="menuitem" onclick={() => { oncenter(); close(); }}>Re-center</button>
            <button role="menuitem" onclick={() => { onminimap(); close(); }}>Toggle minimap</button>
          {:else}
            <div role="presentation" class="dropdown-label">Navigate to Workspace to manage panels</div>
          {/if}
        </div>
      {/if}
    </div>
  </nav>

  <nav class="tabs">
    <button
      class="tab"
      class:active={page.url.pathname === '/'}
      onclick={() => goto('/')}
    >Workspace</button>
    <button
      class="tab"
      class:active={page.url.pathname === '/settings'}
      onclick={() => goto('/settings')}
    >Settings</button>
  </nav>

  <!-- Account -->
  <button class="account" onclick={onaccount} aria-label="Account">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor"
      stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="12" cy="8" r="4"/>
      <path d="M4 20c0-4 3.6-7 8-7s8 3 8 7"/>
    </svg>
  </button>
</header>

<style>
  .topbar {
    display: flex;
    flex-direction: row;
    align-items: center;
    height: 32px;
    min-height: 32px;
    padding: 0 8px;
    background: #0f0f17;
    border-bottom: 1px solid rgba(255,255,255,0.08);
    user-select: none;
    position: relative;
    z-index: 100;
    flex-shrink: 0;
    overflow: visible;
    white-space: nowrap;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-right: 8px;
    flex-shrink: 0;
  }

  .logo-name {
    font-size: 1rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: #e2e8f0;
    font-family: monospace;
  }

  .menu {
    display: flex;
    flex-direction: row;
    align-items: center;
    height: 100%;
  }

  .menu-item {
    position: relative;
    height: 100%;
    display: flex;
    align-items: center;
  }

  .menu-item > button {
    background: none;
    border: none;
    color: rgba(226,232,240,0.75);
    font-size: 1rem;
    padding: 0 10px;
    height: 100%;
    cursor: pointer;
    white-space: nowrap;
    transition: color 0.15s, background 0.15s;
  }

  .menu-item > button:hover,
  .menu-item.open > button {
    color: #e2e8f0;
    background: rgba(255,255,255,0.07);
  }

  .dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    min-width: 180px;
    background: #1a1a2e;
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 6px;
    padding: 4px;
    margin: 2px 0 0;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
    display: flex;
    flex-direction: column;
    z-index: 200;
  }

  .dropdown-label {
    font-size: 0.85rem;
    color: rgba(226,232,240,0.35);
    /*font-family: system-ui, -apple-system, sans-serif;*/
    padding: 6px 10px 2px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .dropdown button {
    display: flex;
    align-items: center;
    width: 100%;
    background: none;
    border: none;
    color: rgba(226,232,240,0.85);
    font-size: 1rem;
    padding: 6px 10px;
    text-align: left;
    cursor: pointer;
    border-radius: 4px;
    white-space: nowrap;
    transition: background 0.1s, color 0.1s;
  }

  .dropdown button:hover {
    background: rgba(255,255,255,0.08);
    color: #e2e8f0;
  }

  .check {
    display: inline-block;
    width: 18px;
    font-size: 0.85rem;
    color: #7c9ef8;
    flex-shrink: 0;
  }

  .sep {
    height: 1px;
    background: rgba(255,255,255,0.08);
    margin: 4px 0;
  }

  .tabs {
    display: flex;
    align-items: center;
    height: 100%;
    gap: 2px;
    margin-left: auto;
    padding: 0 8px;
  }

  .tab {
    background: none;
    border: none;
    color: rgba(226,232,240,0.45);
    font-size: 0.92rem;
    font-family: monospace;
    padding: 0 10px;
    height: 100%;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: color 0.15s, border-color 0.15s;
  }

  .tab:hover {
    color: rgba(226,232,240,0.8);
  }

  .tab.active {
    color: #e2e8f0;
    border-bottom-color: rgb(225,80,35);
  }

  .account {
    background: none;
    border: 1px solid rgba(255,255,255,0.15);
    border-radius: 50%;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: rgba(226,232,240,0.7);
    cursor: pointer;
    flex-shrink: 0;
    transition: border-color 0.15s, color 0.15s;
  }

  .account:hover {
    border-color: rgba(255,255,255,0.35);
    color: #e2e8f0;
  }
</style>
