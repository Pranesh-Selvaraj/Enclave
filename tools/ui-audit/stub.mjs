// Stubbed Tauri IPC backend shared by the UI audit and the local demo
// (tools/ui-audit/demo.mjs). Not shipped; the real app always runs in Tauri.
export function stubSource() {
	return `(() => {
  const now = Date.now();
  const iso = (h) => new Date(now - h * 3600000).toISOString();
  let docs = [
    { id: 'doc-1', title: 'Product roadmap', created_at: iso(400), updated_at: iso(1), is_favorite: true, is_archived: false, folder_id: 'f-1' },
    { id: 'doc-2', title: 'Meeting notes — design sync', created_at: iso(300), updated_at: iso(5), is_favorite: false, is_archived: false, folder_id: 'f-1' },
    { id: 'doc-3', title: 'Reading list', created_at: iso(200), updated_at: iso(30), is_favorite: true, is_archived: false, folder_id: null },
    { id: 'doc-4', title: 'Vault architecture', created_at: iso(150), updated_at: iso(70), is_favorite: false, is_archived: false, folder_id: null },
    { id: 'doc-5', title: 'Weekly review — sprint 12', created_at: iso(100), updated_at: iso(80), is_favorite: false, is_archived: false, folder_id: null },
    { id: 'doc-rich', title: 'Everything bagel', created_at: iso(20), updated_at: iso(0), is_favorite: false, is_archived: false, folder_id: null }
  ];
  const archived = [{ id: 'doc-x', title: 'Old draft', created_at: iso(500), updated_at: iso(400), is_favorite: false, is_archived: true, folder_id: null }];
  let createdCount = 0;
  const folders = [{ id: 'f-1', name: 'Projects', created_at: iso(600) }];
  const tags = [
    { doc_id: 'doc-1', tags: ['product', 'planning'] },
    { doc_id: 'doc-rich', tags: ['kitchen-sink'] },
    { doc_id: 'doc-2', tags: ['design'] },
    { doc_id: 'doc-3', tags: ['reading'] }
  ];
  const docContent = { type: 'doc', content: [
    { type: 'heading', attrs: { level: 1 }, content: [{ type: 'text', text: 'Roadmap Q4' }] },
    { type: 'paragraph', content: [{ type: 'text', text: 'Theme: make sync feel instant and the editor feel native on every screen.' }] },
    { type: 'heading', attrs: { level: 2 }, content: [{ type: 'text', text: 'Now' }] },
    { type: 'bulletList', content: [
      { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Incremental sync on the wire' }] }] },
      { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Android background service' }] }] },
      { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'UI rework pass on desktop + mobile' }] }] }
    ]},
    { type: 'paragraph', content: [{ type: 'text', text: 'Next: split view polish and the graph touch targets.' }] }
  ]};
  const richContent = { type: 'doc', content: [
    { type: 'heading', attrs: { level: 1 }, content: [{ type: 'text', text: 'Everything bagel' }] },
    { type: 'paragraph', content: [
      { type: 'text', marks: [{ type: 'bold' }], text: 'Bold' }, { type: 'text', text: ', ' },
      { type: 'text', marks: [{ type: 'italic' }], text: 'italic' }, { type: 'text', text: ', ' },
      { type: 'text', marks: [{ type: 'strike' }], text: 'strike' }, { type: 'text', text: ', ' },
      { type: 'text', marks: [{ type: 'code' }], text: 'code' }, { type: 'text', text: ' and a ' },
      { type: 'text', marks: [{ type: 'link', attrs: { href: 'https://example.com/a/very/long/link' } }], text: 'link' }, { type: 'text', text: '.' }
    ]},
    { type: 'bulletList', content: [
      { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'First bullet with a fairly long sentence to stress wrapping at 360px width.' }] }] },
      { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Second bullet' }] }] }
    ]},
    { type: 'taskList', content: [
      { type: 'taskItem', attrs: { checked: false }, content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Unchecked task' }] }] },
      { type: 'taskItem', attrs: { checked: true }, content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Checked task' }] }] }
    ]},
    { type: 'callout', attrs: { type: 'info' }, content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Callout with a long body that must wrap inside the rounded box without overflowing on a phone.' }] }] },
    { type: 'toggleBlock', attrs: { collapsed: false }, content: [
      { type: 'toggleSummary', content: [{ type: 'text', text: 'Toggle title' }] },
      { type: 'paragraph', content: [{ type: 'text', text: 'Toggle body content.' }] }
    ]},
    { type: 'codeBlock', attrs: { language: 'js' }, content: [{ type: 'text', text: 'const veryLongLine = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";' }] },
    { type: 'table', content: [
      { type: 'tableRow', content: [
        { type: 'tableHeader', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Column one' }] }] },
        { type: 'tableHeader', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Column two' }] }] },
        { type: 'tableHeader', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Column three' }] }] }
      ]},
      { type: 'tableRow', content: [
        { type: 'tableCell', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'A cell with a long value that should wrap or scroll' }] }] },
        { type: 'tableCell', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'short' }] }] },
        { type: 'tableCell', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'another' }] }] }
      ]}
    ]},
    { type: 'database', attrs: { data: JSON.stringify({ columns: [{ id: 'c1', name: 'Task', type: 'text' }, { id: 'c2', name: 'Status', type: 'select' }, { id: 'c3', name: 'Notes', type: 'text' }], rows: [ { id: 'r1', cells: { c1: 'Ship the mobile drawer', c2: 'Done', c3: 'long note' } }, { id: 'r2', cells: { c1: 'Audit editor components', c2: 'Doing', c3: '' } } ], density: 'comfortable' }) } },
    { type: 'image', attrs: { src: 'https://example.com/pic.png', alt: 'Placeholder image', caption: 'A caption under the image' } },
    { type: 'bookmark', attrs: { url: 'https://example.com/a/very/long/url/that/should/ellipsize/nicely', title: 'A bookmark card title' } },
    { type: 'horizontalRule' }
  ]};
  const whiteboard = { elements: [
    { id: 'sh-1', type: 'sticky', x: 60, y: 80, w: 180, h: 120, text: 'Ship incremental sync' },
    { id: 'sh-2', type: 'rect', x: 320, y: 130, w: 200, h: 110, text: 'Android parity' },
    { id: 'sh-3', type: 'ellipse', x: 600, y: 90, w: 140, h: 140, text: '' },
    { id: 'sh-4', type: 'arrow', x: 245, y: 145, w: 70, h: 15, text: '' },
    { id: 'sh-5', type: 'text', x: 600, y: 260, w: 180, h: 32, text: 'Q4 themes' }
  ] };
  let cbId = 0;
  const callbacks = {};
  // Persistent-ish app settings (AI config, Android widget prefs) live in the
  // vault through get_setting/set_setting — keep them in memory for the tour.
  const settingsMap = {};
  const route = (cmd, args) => {
    switch (cmd) {
      case 'is_vault_initialized': return !new URLSearchParams(location.search).has('fresh');
      // The tour renders an unlocked vault, so shared-core consumers
      // (capture/widget inline guard) see it as open.
      case 'is_vault_unlocked':
        // Root layout skips the guard when the shared core is already open.
        // The tour wants the guard on the main routes, but capture/widget
        // routes are inline-guarded and should render their unlocked form.
        // The locked_capture query param exercises the locked password-vault
        // path (#22): the capture window must show the inline unlock guard.
        if (new URLSearchParams(location.search).has('locked_capture')) return false;
        return location.pathname.startsWith('/capture') || location.pathname.startsWith('/widget');
      // VaultGuard uses the seed-phrase path in the tour; the capture/widget
      // routes auto-unlock with a stored key and never decrypt it.
      case 'load_vault_key': return location.pathname.startsWith('/capture') || location.pathname.startsWith('/widget')
        ? [1, 2, 3, 4]
        : Promise.reject(new Error('seed-phrase vault'));
      case 'unlock_vault': return null;
      case 'create_document': {
        const d = { id: 'doc-new-' + (++createdCount), title: (args && args.title) || 'Untitled', created_at: iso(0), updated_at: iso(0), is_favorite: false, is_archived: false, folder_id: null };
        docs.unshift(d);
        return d;
      }
      case 'archive_document': {
        const i = docs.findIndex((d) => d.id === (args && args.id));
        if (i >= 0) { archived.unshift({ ...docs[i], is_archived: true }); docs.splice(i, 1); }
        return null;
      }
      case 'restore_document': {
        const i = archived.findIndex((d) => d.id === (args && args.id));
        if (i >= 0) { docs.unshift({ ...archived[i], is_archived: false }); archived.splice(i, 1); }
        return null;
      }
      case 'delete_document': {
        const i = archived.findIndex((d) => d.id === (args && args.id));
        if (i >= 0) archived.splice(i, 1);
        return null;
      }
      case 'toggle_favorite': {
        const d = docs.find((x) => x.id === (args && args.id)) || archived.find((x) => x.id === (args && args.id));
        if (d) d.is_favorite = !d.is_favorite;
        return null;
      }
      case 'update_document_title': {
        const d = docs.find((x) => x.id === (args && args.id));
        if (d) { d.title = args.title; d.updated_at = iso(0); }
        return d || null;
      }
      case 'create_folder': {
        const f = { id: 'f-' + (++createdCount), name: (args && args.name) || 'New folder', created_at: iso(0) };
        folders.push(f);
        return f;
      }
      case 'get_document_list': return docs;
      case 'get_archived_documents': return archived;
      case 'get_folders': return folders;
      case 'get_all_tags': return tags;
      case 'get_page_list': return docs.map((d) => ({ id: d.id, title: d.title }));
      case 'get_document': return docs.find((d) => d.id === (args && args.id)) || docs[0];
      case 'get_blocks': {
        const id = args && args.documentId;
        const t = (tags.find((x) => x.doc_id === id) || {}).tags || [];
        const content = id === 'doc-rich' ? richContent : docContent;
        const blocks = [
          { id: id + '-content', document_id: id, type: 'doc', content, sort_order: 0, created_at: iso(1), updated_at: iso(1) },
          { id: id + '-tags', document_id: id, type: 'tags', content: { tags: t }, sort_order: 2, created_at: iso(1), updated_at: iso(1) },
          { id: id + '-meta', document_id: id, type: 'meta', content: { icon: '🚀', cover: id === 'doc-1' ? 'grad-2' : '' }, sort_order: 3, created_at: iso(1), updated_at: iso(1) }
        ];
        if (id === 'doc-1') blocks.push({ id: id + '-wb', document_id: id, type: 'whiteboard', content: whiteboard, sort_order: 4, created_at: iso(1), updated_at: iso(1) });
        return blocks;
      }
      case 'get_backlinks': return [{ doc_id: 'doc-2', doc_title: 'Meeting notes — design sync', block_content: 'mentioned [[Product roadmap]] while planning Q4' }];
      case 'find_relation_backlinks': return [];
      case 'search_all': return [];
      case 'search_embeddings': return [];
      case 'get_setting': return (args && args.key && settingsMap[args.key] !== undefined) ? settingsMap[args.key] : null;
      case 'set_setting': {
        if (args && args.key) settingsMap[args.key] = args.value;
        return null;
      }
      case 'network_status': return { local_peer_id: 'peer-abc123', local_host: '192.168.1.42', running: false, port: 4242, peers: [], last_sync_at: null };
      case 'app_version': return '1.10.1';
      case 'backup_vault': return '/home/user/.local/share/com.enclave.app/exports/enclave-backup-20260101-120000.db';
      case 'check_for_update': return {
        current_version: '1.10.1', latest_version: '1.11.0', update_available: true,
        notes: ['## What changed — faster sync handshake, Android widget fixes', '- Sync reconnects without a restart', '- Android widgets refresh after an edit'].join(String.fromCharCode(10)),
        asset_name: 'Enclave_1.11.0_amd64.deb', asset_url: 'https://example.invalid/Enclave_1.11.0_amd64.deb', asset_size: 23456789
      };
      case 'download_update': return '/tmp/enclave-update/Enclave_1.11.0_amd64.deb';
      case 'install_update': return null;
      case 'plugin:event|listen': return 1;
      case 'plugin:event|unlisten': return null;
      case 'export_file': return '/tmp/enclave-export';
      default: return null;
    }
  };
  window.__TAURI_INTERNALS__ = {
    callbacks,
    transformCallback: (cb) => { const id = ++cbId; callbacks[id] = cb; return id; },
    unregisterCallback: (id) => { delete callbacks[id]; },
    convertFileSrc: (p) => p,
    invoke: (cmd, args) => Promise.resolve(route(cmd, args)),
    metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } }
  };
})();`;
}
