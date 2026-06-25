<script lang="ts">
  import '../app.css';

  let { children } = $props();

  interface NavItem {
    label: string;
    icon: string;
    href: string;
  }

  const navItems: NavItem[] = [
    { label: 'All Notes', icon: '📄', href: '/' },
    { label: 'Favorites', icon: '⭐', href: '/favorites' },
    { label: 'Tags', icon: '🏷️', href: '/tags' },
    { label: 'Archive', icon: '📦', href: '/archive' },
  ];

  let sidebarOpen = $state(true);
  let syncStatus = $state<'offline' | 'scanning' | 'synced'>('offline');
</script>

<div class="app-shell">
  <!-- Sidebar -->
  <aside class="sidebar" class:collapsed={!sidebarOpen}>
    <div class="sidebar-header">
      <div class="logo">
        <span class="logo-icon">🔒</span>
        {#if sidebarOpen}
          <h1 class="logo-text">Athernote</h1>
        {/if}
      </div>
      <button
        class="sidebar-toggle"
        onclick={() => sidebarOpen = !sidebarOpen}
        aria-label="Toggle sidebar"
      >
        {sidebarOpen ? '◀' : '▶'}
      </button>
    </div>

    {#if sidebarOpen}
      <nav class="sidebar-nav">
        {#each navItems as item}
          <a href={item.href} class="nav-item">
            <span class="nav-icon">{item.icon}</span>
            <span class="nav-label">{item.label}</span>
          </a>
        {/each}
      </nav>

      <div class="sidebar-footer">
        <div class="sync-status" class:synced={syncStatus === 'synced'} class:scanning={syncStatus === 'scanning'}>
          <span class="sync-dot"></span>
          <span class="sync-label">
            {#if syncStatus === 'offline'}
              Offline
            {:else if syncStatus === 'scanning'}
              Scanning network...
            {:else}
              Synced
            {/if}
          </span>
        </div>
        <p class="sidebar-version">v0.1.0 — local-first</p>
      </div>
    {/if}
  </aside>

  <!-- Main Content -->
  <main class="main-content">
    {@render children()}
  </main>
</div>

<style>
  .app-shell {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    width: 240px;
    min-width: 240px;
    background-color: var(--color-surface);
    border-right: 1px solid var(--color-border);
    transition: width 0.2s ease, min-width 0.2s ease;
  }

  .sidebar.collapsed {
    width: 56px;
    min-width: 56px;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 12px;
    border-bottom: 1px solid var(--color-border);
  }

  .sidebar.collapsed .sidebar-header {
    justify-content: center;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .logo-icon {
    font-size: 22px;
  }

  .logo-text {
    font-size: 16px;
    font-weight: 700;
    letter-spacing: -0.02em;
    margin: 0;
    white-space: nowrap;
  }

  .sidebar-toggle {
    background: none;
    border: none;
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: 12px;
    padding: 4px 6px;
    border-radius: 4px;
    transition: color 0.15s, background-color 0.15s;
  }

  .sidebar-toggle:hover {
    color: var(--color-text);
    background-color: rgba(255, 255, 255, 0.06);
  }

  .sidebar-nav {
    flex: 1;
    padding: 12px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-radius: 6px;
    color: var(--color-text-muted);
    text-decoration: none;
    font-size: 14px;
    transition: color 0.15s, background-color 0.15s;
  }

  .nav-item:hover {
    color: var(--color-text);
    background-color: rgba(255, 255, 255, 0.05);
  }

  .nav-icon {
    font-size: 16px;
    width: 24px;
    text-align: center;
  }

  .sidebar-footer {
    padding: 12px 16px;
    border-top: 1px solid var(--color-border);
  }

  .sync-status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--color-text-muted);
  }

  .sync-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: #666;
    transition: background-color 0.3s;
  }

  .sync-status.scanning .sync-dot {
    background-color: #f0c040;
    animation: pulse 1.5s infinite;
  }

  .sync-status.synced .sync-dot {
    background-color: var(--color-success);
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .sidebar-version {
    margin: 8px 0 0;
    font-size: 10px;
    color: var(--color-text-muted);
    opacity: 0.5;
  }

  .main-content {
    flex: 1;
    overflow-y: auto;
    padding: 0;
    background-color: var(--color-bg);
  }
</style>
