<script lang="ts">
  let noteCount = $state(0);
  let notePreview = $state('');

  function createNote() {
    noteCount++;
    notePreview = `Note #${noteCount} created.\n\nThis is a local-first, encrypted note.\nAll data stays on your device.`;
  }

  function clearNotes() {
    noteCount = 0;
    notePreview = '';
  }
</script>

<div class="page-home">
  <div class="page-header">
    <div>
      <h2 class="page-title">All Notes</h2>
      <p class="page-subtitle">Encrypted, local-first, zero-knowledge</p>
    </div>
    <div class="header-actions">
      <button class="btn btn-primary" onclick={createNote}>
        + New Note
      </button>
    </div>
  </div>

  <div class="page-body">
    {#if noteCount === 0}
      <div class="empty-state">
        <div class="empty-icon">📝</div>
        <h3>No notes yet</h3>
        <p>
          Your notes are encrypted with AES-256-GCM and stored locally.
          Sync happens P2P over your local network — no cloud, no servers.
        </p>
        <button class="btn btn-primary" onclick={createNote}>
          Create your first note
        </button>
      </div>
    {:else}
      <div class="notes-list">
        <div class="note-card">
          <div class="note-card-header">
            <span class="note-title">Welcome to Athernote</span>
            <span class="note-status encrypted">🔒 Encrypted</span>
          </div>
          <pre class="note-content">{notePreview}</pre>
          <div class="note-card-footer">
            <span class="note-meta">Just now</span>
            <span class="note-meta">0 words</span>
          </div>
        </div>
      </div>

      <div class="page-footer-actions">
        <button class="btn btn-secondary" onclick={createNote}>
          + Add another note
        </button>
        <button class="btn btn-ghost" onclick={clearNotes}>
          Clear all
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .page-home {
    max-width: 800px;
    margin: 0 auto;
    padding: 32px;
  }

  .page-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 32px;
    padding-bottom: 20px;
    border-bottom: 1px solid var(--color-border);
  }

  .page-title {
    font-size: 22px;
    font-weight: 700;
    margin: 0 0 4px;
    letter-spacing: -0.02em;
  }

  .page-subtitle {
    font-size: 13px;
    color: var(--color-text-muted);
    margin: 0;
  }

  .header-actions {
    display: flex;
    gap: 8px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 80px 40px;
    border: 2px dashed var(--color-border);
    border-radius: 12px;
  }

  .empty-icon {
    font-size: 48px;
    margin-bottom: 16px;
  }

  .empty-state h3 {
    font-size: 18px;
    font-weight: 600;
    margin: 0 0 8px;
  }

  .empty-state p {
    max-width: 420px;
    color: var(--color-text-muted);
    font-size: 14px;
    line-height: 1.6;
    margin: 0 0 24px;
  }

  .notes-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 24px;
  }

  .note-card {
    background-color: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    padding: 20px;
  }

  .note-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .note-title {
    font-size: 16px;
    font-weight: 600;
  }

  .note-status {
    font-size: 11px;
    padding: 3px 8px;
    border-radius: 4px;
    background-color: rgba(124, 111, 240, 0.12);
    color: var(--color-accent);
  }

  .note-content {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 13px;
    line-height: 1.7;
    color: var(--color-text);
    margin: 0 0 16px;
    white-space: pre-wrap;
    background: none;
    border: none;
    padding: 0;
  }

  .note-card-footer {
    display: flex;
    gap: 16px;
  }

  .note-meta {
    font-size: 12px;
    color: var(--color-text-muted);
  }

  .page-footer-actions {
    display: flex;
    gap: 8px;
    justify-content: center;
  }

  /* Button system */
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
    transition: background-color 0.15s, border-color 0.15s, color 0.15s;
  }

  .btn-primary {
    background-color: var(--color-accent);
    color: #fff;
    border-color: var(--color-accent);
  }

  .btn-primary:hover {
    background-color: var(--color-accent-hover);
    border-color: var(--color-accent-hover);
  }

  .btn-secondary {
    background-color: rgba(255, 255, 255, 0.06);
    color: var(--color-text);
    border-color: var(--color-border);
  }

  .btn-secondary:hover {
    background-color: rgba(255, 255, 255, 0.1);
  }

  .btn-ghost {
    background: none;
    color: var(--color-text-muted);
  }

  .btn-ghost:hover {
    color: var(--color-text);
    background-color: rgba(255, 255, 255, 0.05);
  }
</style>
