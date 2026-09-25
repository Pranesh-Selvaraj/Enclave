#!/usr/bin/env node
// Enclave UI screenshot + layout-audit engine.
//
// Renders the real Enclave frontend (SvelteKit) in headless Chromium with the
// Tauri IPC stubbed, then walks scenarios and writes PNGs plus a manifest of
// audit findings. Run through tools/ui-audit/run.mjs (which serves the built
// frontend and generates the HTML report), or directly with --url against a
// dev server. No extra npm dependencies — it drives Chrome over CDP and uses
// Node's built-in fetch/WebSocket.
//
// Usage:
//   node shot.mjs --url http://127.0.0.1:4173 --profiles phone,desktop --scenario all
//
// Options:
//   --url <url>            Frontend URL (required)
//   --out <dir>            Output dir (default ./shots)
//   --profiles <list>      Comma list of profile names (default: desktop)
//   --scenario <name>      Scenario or "all" (default: all)
//   --theme dark|light     Emulated prefers-color-scheme (default: dark)
//   --setting k=v          Repeatable; seeds localStorage enclave-settings
//                          (e.g. --setting uiScale=compact --setting trueBlack=true)
//   --chrome <path>        Chromium/Chrome binary
//   --keep                 Keep the browser open after the run
//   --verbose              Log console errors from the page
import { spawn } from 'node:child_process';
import { existsSync, readdirSync, statSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const MNEMONIC =
	'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';

// ── Device profiles ──────────────────────────────────────────────────────────
const PROFILES = {
	desktop: { width: 1440, height: 900, dpr: 1, mobile: false },
	'desktop-wide': { width: 1920, height: 1080, dpr: 1, mobile: false },
	phone: { width: 412, height: 915, dpr: 2.625, mobile: true }, // Pixel 8 class
	'phone-small': { width: 360, height: 740, dpr: 3, mobile: true }, // 360dp budget class
	tablet: { width: 800, height: 1280, dpr: 2, mobile: true },
	fold: { width: 673, height: 841, dpr: 2, mobile: true }, // unfolded inner screen
};

// ── Args ─────────────────────────────────────────────────────────────────────
function parseArgs(argv) {
	const args = { profiles: 'desktop', scenario: 'all', theme: 'dark', out: join(HERE, 'shots'), settings: {} };
	for (let i = 0; i < argv.length; i++) {
		const a = argv[i];
		const next = () => argv[++i];
		if (a === '--url') args.url = next();
		else if (a === '--out') args.out = next();
		else if (a === '--profiles') args.profiles = next();
		else if (a === '--scenario') args.scenario = next();
		else if (a === '--theme') args.theme = next();
		else if (a === '--chrome') args.chrome = next();
		else if (a === '--setting') {
			const [k, ...v] = next().split('=');
			args.settings[k] = v.join('=');
		} else if (a === '--keep') args.keep = true;
		else if (a === '--verbose') args.verbose = true;
		else if (a === '--help' || a === '-h') args.help = true;
		else throw new Error(`unknown argument: ${a}`);
	}
	return args;
}

function findChrome(explicit) {
	const candidates = [];
	if (explicit) candidates.push(explicit);
	if (process.env.CHROME_PATH) candidates.push(process.env.CHROME_PATH);
	for (const root of [join(process.env.HOME ?? '', '.cache/ms-playwright'), join(process.env.HOME ?? '', '.cache/puppeteer')]) {
		if (!existsSync(root)) continue;
		for (const dir of readdirSync(root).sort().reverse()) {
			for (const rel of [
				join(root, dir, 'chrome-linux64', 'chrome'),
				join(root, dir, 'chrome-linux', 'chrome'),
				join(root, dir, 'chrome-headless-shell-linux64', 'chrome-headless-shell'),
			]) {
				if (existsSync(rel)) candidates.push(rel);
			}
		}
	}
	candidates.push('/usr/bin/chromium', '/usr/bin/chromium-browser', '/usr/bin/google-chrome');
	const found = candidates.find((c) => c && existsSync(c));
	if (!found) throw new Error('no Chromium found — pass --chrome <path> or set CHROME_PATH');
	return found;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── Minimal CDP client ───────────────────────────────────────────────────────
class CDP {
	constructor(socket) {
		this.ws = socket;
		this.seq = 0;
		this.pending = new Map();
		this.listeners = new Map();
		socket.onmessage = (ev) => {
			const msg = JSON.parse(ev.data);
			if (msg.id) {
				const p = this.pending.get(msg.id);
				if (!p) return;
				this.pending.delete(msg.id);
				if (msg.error) p.reject(new Error(msg.error.message));
				else p.resolve(msg.result);
			} else {
				for (const fn of this.listeners.get(msg.method) ?? []) fn(msg.params);
			}
		};
	}
	send(method, params = {}) {
		const id = ++this.seq;
		return new Promise((resolve, reject) => {
			this.pending.set(id, { resolve, reject });
			this.ws.send(JSON.stringify({ id, method, params }));
		});
	}
	once(method, timeout = 30000) {
		return new Promise((resolve, reject) => {
			const arr = this.listeners.get(method) ?? [];
			const t = setTimeout(() => reject(new Error(`timeout ${method}`)), timeout);
			const fn = (p) => {
				clearTimeout(t);
				this.listeners.set(method, arr.filter((x) => x !== fn));
				resolve(p);
			};
			arr.push(fn);
			this.listeners.set(method, arr);
		});
	}
}

// ── Fake vault / backend ─────────────────────────────────────────────────────
// Every command the UI reads during a tour. Mutations resolve to null.
function stubSource() {
	return `(() => {
  const now = Date.now();
  const iso = (h) => new Date(now - h * 3600000).toISOString();
  let docs = [
    { id: 'doc-1', title: 'Product roadmap', created_at: iso(400), updated_at: iso(1), is_favorite: true, is_archived: false, folder_id: 'f-1' },
    { id: 'doc-2', title: 'Meeting notes — design sync', created_at: iso(300), updated_at: iso(5), is_favorite: false, is_archived: false, folder_id: 'f-1' },
    { id: 'doc-3', title: 'Reading list', created_at: iso(200), updated_at: iso(30), is_favorite: true, is_archived: false, folder_id: null },
    { id: 'doc-4', title: 'Vault architecture', created_at: iso(150), updated_at: iso(70), is_favorite: false, is_archived: false, folder_id: null },
    { id: 'doc-5', title: 'Weekly review — sprint 12', created_at: iso(100), updated_at: iso(80), is_favorite: false, is_archived: false, folder_id: null }
  ];
  const archived = [{ id: 'doc-x', title: 'Old draft', created_at: iso(500), updated_at: iso(400), is_favorite: false, is_archived: true, folder_id: null }];
  let createdCount = 0;
  const folders = [{ id: 'f-1', name: 'Projects', created_at: iso(600) }];
  const tags = [
    { doc_id: 'doc-1', tags: ['product', 'planning'] },
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
        const blocks = [
          { id: id + '-content', document_id: id, type: 'doc', content: docContent, sort_order: 0, created_at: iso(1), updated_at: iso(1) },
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

function settingsSeedSource(settings) {
	const has = Object.keys(settings).length > 0;
	if (!has) return '';
	return `(() => {
  try {
    const s = JSON.parse(localStorage.getItem('enclave-settings') || '{}');
    const patch = ${JSON.stringify(settings)};
    for (const k of Object.keys(patch)) {
      let v = patch[k];
      if (v === 'true') v = true; else if (v === 'false') v = false;
      s[k] = v;
    }
    localStorage.setItem('enclave-settings', JSON.stringify(s));
  } catch {}
})();`;
}

// ── Session helpers ──────────────────────────────────────────────────────────
class Session {
	constructor(cdp, { url, out, profile, profileName, theme, verbose }) {
		Object.assign(this, { cdp, url, out, profile, profileName, theme, verbose });
		this.base = url.replace(/\/$/, '');
		this.issues = [];
	}

	check(name, condition, detail = '') {
		const ok = !!condition;
		if (!ok) {
			this.issues.push({ name, detail: String(detail) });
			console.error(`  AUDIT FAIL ${this.profileName}/${name}: ${detail}`);
		}
		return ok;
	}

	async eval(expression, awaitPromise = false) {
		const res = await this.cdp.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise });
		if (res.exceptionDetails) {
			throw new Error(`eval failed: ${res.exceptionDetails.exception?.description ?? res.exceptionDetails.text}`);
		}
		return res.result.value;
	}

	async waitFor(fn, timeout = 20000, label = 'condition') {
		const start = Date.now();
		while (Date.now() - start < timeout) {
			if (await this.eval(`!!(${fn.toString()})()`)) return;
			await sleep(150);
		}
		throw new Error(`timeout waiting for ${label}`);
	}

	async boot(url = `${this.base}/`) {
		const loaded = this.cdp.once('Page.loadEventFired');
		await this.cdp.send('Page.navigate', { url });
		await loaded;
		await sleep(300);
	}

	async click(selector) {
		await this.eval(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); if (!el) return false; el.click(); return true; })()`);
	}

	async clickText(text) {
		await this.eval(
			`(() => { const el = [...document.querySelectorAll('button, a')].find((b) => b.textContent.trim() === ${JSON.stringify(text)}); if (!el) return false; el.click(); return true; })()`,
		);
	}

	async fill(selector, value) {
		await this.eval(`(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return false;
      const proto = el.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
      Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, ${JSON.stringify(value)});
      el.dispatchEvent(new Event('input', { bubbles: true }));
      return true;
    })()`);
	}

	async key(key, code = key, vk = 0, modifiers = 0) {
		const base = { key, code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers };
		await this.cdp.send('Input.dispatchKeyEvent', { type: 'keyDown', ...base });
		await this.cdp.send('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
	}

	/** Ctrl+K etc. — CDP modifier bit 2 = Ctrl, bit 4 = Meta. */
	async keyWithCtrl(key, code = key, vk = 0) {
		await this.key(key, code, vk, 2);
	}

	async shot(name) {
		await this.eval('document.fonts.ready.then(() => true)', true).catch(() => {});
		await sleep(250);
		const { data } = await this.cdp.send('Page.captureScreenshot', { format: 'png' });
		const file = join(this.out, `${this.profileName}-${name}.png`);
		await writeFile(file, Buffer.from(data, 'base64'));
		console.log(`  ${this.profileName.padEnd(14)} ${name}`);
	}

	async readyVault() {
		await this.boot(`${this.base}/`);
		await this.waitFor(() => document.querySelector('.vault-card'), 25000, 'vault card');
	}

	async unlock() {
		await this.readyVault();
		// The whiteboard remembers the last view per page; the tour always
		// starts on the paper view or later scenarios inherit whiteboard mode.
		try {
			await this.eval(`(() => { for (let i = localStorage.length - 1; i >= 0; i--) { const k = localStorage.key(i); if (k && k.startsWith('enclave-mode-')) localStorage.removeItem(k); } return true; })()`);
		} catch { /* ignore */ }
		// `.vault-card` also matches the loading/checking spinner — wait for the
		// actual unlock form before typing (otherwise fill() is a silent no-op).
		await this.waitFor(() => document.querySelector('.seed-input'), 30000, 'seed input');
		await this.fill('.seed-input', MNEMONIC);
		await this.clickText('Unlock');
		// Seed-phrase unlock offers a password setup step — skip it.
		await this.waitFor(
			() => [...document.querySelectorAll('button')].some((b) => b.textContent.trim() === 'Skip for now'),
			90000,
			'password setup step',
		);
		await this.clickText('Skip for now');
		await this.waitFor(() => document.querySelector('.sidebar'), 25000, 'app shell');
		await sleep(400);
	}

	async openDoc(id = 'doc-1') {
		// SPA navigation from the unlocked shell — a fresh boot on the route
		// would show the vault guard again.
		await this.click(`a[href="/${id}"]`);
		await this.waitFor(() => document.querySelector('.document-page'), 25000, 'document page');
		await sleep(900); // tiptap mount
	}
}

// ── Settings tour helpers ────────────────────────────────────────────────────
async function openSettings(s) {
	if (s.profile.mobile) {
		await s.eval(
			`(() => { const tab = [...document.querySelectorAll('.bottom-nav .nav-tab')].find((b) => b.textContent.includes('Settings')); if (tab) tab.click(); return !!tab; })()`,
		);
	}
	// Tablets use the desktop sidebar/menu even though they are mobile profiles.
	let opened = await s.eval(`document.querySelector('.settings-panel') !== null`);
	if (!opened && s.profile.mobile) {
		// Doc pages render no bottom nav — open the drawer and use its button.
		await s.eval(`document.querySelector('.mobile-topbar [title="Menu"]')?.click()`);
		await sleep(350);
	}
	if (!opened) {
		await s.eval(
			`(() => { const b = [...document.querySelectorAll('.sidebar .icon-btn, .mini-nav .mini-btn')].find((x) => (x.title || '').includes('Settings')); if (b) b.click(); return !!b; })()`,
		);
	}
	await s.waitFor(() => document.querySelector('.settings-panel'), 10000, 'settings panel');
	await sleep(350);
}

async function clickSeg(s, label, option) {
	return s.eval(`(() => {
    const rows = [...document.querySelectorAll('.settings-panel .setting-row')];
    const row = rows.find((r) => r.querySelector(':scope > span')?.textContent.trim() === ${JSON.stringify(label)});
    if (!row) return 'no-row';
    const btn = [...row.querySelectorAll('.seg')].find((b) => b.textContent.trim().toLowerCase() === ${JSON.stringify(option.toLowerCase())});
    if (!btn) return 'no-option';
    btn.click();
    return 'ok';
  })()`);
}

async function toggleSwitch(s, label) {
	return s.eval(`(() => {
    const rows = [...document.querySelectorAll('.settings-panel .setting-row')];
    const row = rows.find((r) => r.querySelector(':scope > span')?.textContent.trim() === ${JSON.stringify(label)});
    if (!row) return 'no-row';
    const input = row.querySelector('input[type=checkbox]');
    if (!input) return 'no-switch';
    input.click();
    return 'ok';
  })()`);
}

async function panelMetrics(s) {
	return s.eval(`(() => {
    const p = document.querySelector('.settings-panel');
    if (!p) return null;
    const pr = p.getBoundingClientRect();
    const bad = [];
    for (const el of p.querySelectorAll('button, input, .swatch')) {
      const r = el.getBoundingClientRect();
      if (r.width === 0 || r.height === 0) continue;
      if (r.right > pr.right + 1 || r.left < pr.left - 1) {
        bad.push({ tag: el.tagName, cls: el.className.toString().slice(0, 30), text: (el.textContent || el.value || '').trim().slice(0, 20), left: Math.round(r.left), right: Math.round(r.right), panelRight: Math.round(pr.right) });
      }
    }
    return { scrollW: p.scrollWidth, clientW: p.clientWidth, docW: document.documentElement.scrollWidth, winW: window.innerWidth, bad };
  })()`);
}

// Every appearance option, in panel order. `attr` is the DOM marker the
// theme module is expected to apply; `value` is the expected marker value.
const SETTINGS_MATRIX = [
	{ label: 'Theme', key: 'theme', options: ['Auto', 'Light', 'Dark'], attr: 'data-theme', value: (o) => o.toLowerCase() },
	{ label: 'Background', key: 'bg', options: ['Matte', 'Soft glow', 'Glass'], attr: 'data-bg', value: (o) => (o === 'Matte' ? 'matte' : o === 'Soft glow' ? 'soft' : 'glassy') },
	{ label: 'Corners', key: 'corners', options: ['Standard', 'Rounded', 'Rounder'], attr: 'data-corners', value: (o) => o.toLowerCase() },
	{ label: 'UI size', key: 'uiscale', options: ['Compact', 'Regular', 'Large', 'Xlarge'], attr: 'data-ui-scale', value: (o) => o.toLowerCase() },
	{ label: 'Font', key: 'font', options: ['Inter', 'System', 'Serif', 'Mono'], attr: 'data-font', value: (o) => o.toLowerCase() },
	{ label: 'Editor font size', key: 'fontsize', options: ['S', 'M', 'L', 'XL'], attr: 'data-font-size', value: (o) => o.toLowerCase() },
	{ label: 'Page width', key: 'pagewidth', options: ['Compact', 'Wide', 'Full'], attr: 'data-page-width', value: (o) => o.toLowerCase() },
	{ label: 'Interface density', key: 'density', options: ['Narrow', 'Normal', 'Wide'], attr: 'data-density', value: (o) => o.toLowerCase() },
];

async function setSwitch(s, label, on) {
	return s.eval(`(() => {
    const rows = [...document.querySelectorAll('.settings-panel .setting-row')];
    const row = rows.find((r) => r.querySelector(':scope > span')?.textContent.trim() === ${JSON.stringify(label)});
    if (!row) return 'no-row';
    const input = row.querySelector('input[type=checkbox]');
    if (!input) return 'no-switch';
    if (input.checked !== ${on ? 'true' : 'false'}) input.click();
    return 'ok';
  })()`);
}

/** High-signal DOM layout checks shared by the audit scenarios. */
async function domAudit(s, label) {
	const res = await s.eval(`(() => {
    const vw = window.innerWidth, vh = window.innerHeight;
    const out = { label: document.title + ' @ ' + location.pathname, docW: document.documentElement.scrollWidth, vw, issues: [] };
    const visible = (el) => {
      const st = getComputedStyle(el);
      if (st.display === 'none' || st.visibility === 'hidden' || Number(st.opacity) === 0) return false;
      const r = el.getBoundingClientRect();
      return r.width > 0 && r.height > 0;
    };
    if (document.documentElement.scrollWidth > vw + 1) out.issues.push({ type: 'doc-h-overflow', detail: document.documentElement.scrollWidth + ' > ' + vw });
    for (const el of document.querySelectorAll('body *')) {
      if (!visible(el)) continue;
      const st = getComputedStyle(el);
      if (st.position === 'fixed') continue;
      if (!el.matches('button, a, input, textarea, select, h1, h2, h3, .seg, .tree-item, .nav-item, .palette-item, .sheet-item')) continue;
      const r = el.getBoundingClientRect();
      if (r.right <= vw + 4 && r.left >= -4) continue;
      // A control parked off-screen because its whole container is off-screen
      // (closed mobile drawer, collapsed rail) is intentional, not a bug: only
      // flag elements that stick out of an otherwise visible container.
      let ancestor = el.parentElement, containerParked = false;
      while (ancestor && ancestor !== document.body) {
        const ar = ancestor.getBoundingClientRect();
        if (ar.width > 0 && ar.height > 0 && (ar.right <= 0 || ar.left >= vw)) { containerParked = true; break; }
        ancestor = ancestor.parentElement;
      }
      if (containerParked) continue;
      let p = el.parentElement, scrollable = false;
      while (p && p !== document.body) { const ps = getComputedStyle(p); if ((ps.overflowX === 'auto' || ps.overflowX === 'scroll') && p.scrollWidth > p.clientWidth) { scrollable = true; break; } p = p.parentElement; }
      if (scrollable) continue;
      out.issues.push({ type: 'offscreen', tag: el.tagName, cls: (el.className || '').toString().slice(0, 40), text: (el.textContent || '').trim().slice(0, 30), left: Math.round(r.left), right: Math.round(r.right) });
    }
    for (const el of document.querySelectorAll('button, .seg, .tree-item-label, .btn-label, .nav-tab, .page-title, kbd')) {
      if (!visible(el)) continue;
      const st = getComputedStyle(el);
      if (st.whiteSpace !== 'nowrap') continue;
      if (el.clientWidth > 0 && el.scrollWidth > el.clientWidth + 1) out.issues.push({ type: 'clipped-text', tag: el.tagName, cls: (el.className || '').toString().slice(0, 40), text: (el.textContent || '').trim().slice(0, 30), scrollW: el.scrollWidth, clientW: el.clientWidth });
    }
    if (window.matchMedia('(max-width: 768px)').matches) {
      for (const el of document.querySelectorAll('button, a[href], [role=button]')) {
        if (!visible(el)) continue;
        const r = el.getBoundingClientRect();
        if (r.width < 32 || r.height < 32) out.issues.push({ type: 'tiny-target', tag: el.tagName, cls: (el.className || '').toString().slice(0, 40), text: (el.textContent || el.getAttribute('aria-label') || '').trim().slice(0, 30), w: Math.round(r.width), h: Math.round(r.height) });
      }
    }
    return out;
  })()`);
	const summary = {};
	for (const issue of res.issues) {
		const k = issue.type;
		if (!summary[k]) summary[k] = [];
		if (summary[k].length < 8) summary[k].push(issue);
	}
	for (const [type, list] of Object.entries(summary)) {
		s.check(`${label}-${type}`, false, JSON.stringify(list));
	}
	return res;
}

// ── Scenarios ────────────────────────────────────────────────────────────────
const SCENARIOS = {
	async welcome(s) {
		await s.boot(`${s.base}/?fresh`);
		await s.waitFor(() => document.querySelector('.vault-features'), 25000, 'welcome card');
		await s.shot('welcome');
	},
	async unlock(s) {
		await s.readyVault();
		await s.waitFor(() => document.querySelector('.seed-input'), 25000, 'unlock form');
		await s.shot('unlock');
	},
	async home(s) {
		await s.unlock();
		await s.shot('home');
	},
	async doc(s) {
		await s.unlock();
		await s.openDoc('doc-1');
		await s.shot('doc');
	},
	async whiteboard(s) {
		await s.unlock();
		await s.openDoc('doc-1');
		await s.clickText('Whiteboard');
		await s.waitFor(() => document.querySelector('canvas'), 15000, 'whiteboard canvas');
		await sleep(700);
		await s.shot('whiteboard');
	},
	async graph(s) {
		await s.unlock();
		await s.click('a[href="/graph"]');
		await s.waitFor(() => document.querySelector('canvas'), 20000, 'graph canvas');
		await sleep(1200);
		await s.shot('graph');
	},
	async settings(s) {
		await s.unlock();
		await s.eval(
			`(() => { const b = [...document.querySelectorAll('.sidebar .icon-btn, .mini-nav .mini-btn')].find((x) => (x.title || '').includes('Settings')); if (b) b.click(); return !!b; })()`,
		);
		await s.waitFor(() => document.querySelector('.settings-panel'), 10000, 'settings panel');
		await sleep(400);
		await s.shot('settings');
	},
	async palette(s) {
		await s.unlock();
		await s.keyWithCtrl('k', 'KeyK', 75);
		await s.waitFor(() => document.querySelector('.command-palette'), 10000, 'command palette');
		await sleep(400);
		await s.shot('palette');
	},
	async drawer(s) {
		await s.unlock();
		await s.click('.mobile-topbar .topbar-btn');
		await s.waitFor(() => document.querySelector('.sidebar.open'), 10000, 'drawer open');
		await sleep(350);
		await s.shot('drawer');
	},
	async sidebar(s) {
		await s.unlock();
		await s.shot('sidebar');
		const before = await s.eval(`document.querySelector('.sidebar').getBoundingClientRect().width`);
		const pt = await s.eval(
			`(() => { const r = document.querySelector('.sidebar-resizer').getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()`,
		);
		await s.cdp.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: pt.x, y: pt.y, button: 'left', clickCount: 1 });
		for (let i = 1; i <= 8; i++) {
			await s.cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: pt.x + i * 10, y: pt.y, button: 'left' });
			await sleep(20);
		}
		await s.cdp.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: pt.x + 80, y: pt.y, button: 'left', clickCount: 1 });
		await sleep(250);
		const after = await s.eval(`document.querySelector('.sidebar').getBoundingClientRect().width`);
		const saved = Number(await s.eval(`localStorage.getItem('enclave-sidebar-width')`));
		if (!(after > before + 40) || Math.abs(saved - after) > 1.5) {
			throw new Error(`resize failed: before=${before} after=${after} saved=${saved}`);
		}
		console.log(`  sidebar resized ${Math.round(before)} → ${Math.round(after)}px (persisted)`);
		await s.shot('sidebar-resized');
		// Double-click resets to the density preset and clears the saved width.
		const pt2 = { x: pt.x + 80, y: pt.y };
		await s.cdp.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: pt2.x, y: pt2.y, button: 'left', clickCount: 2 });
		await s.cdp.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: pt2.x, y: pt2.y, button: 'left', clickCount: 2 });
		await sleep(250);
		const reset = await s.eval(`document.querySelector('.sidebar').getBoundingClientRect().width`);
		const cleared = await s.eval(`localStorage.getItem('enclave-sidebar-width')`);
		if (Math.abs(reset - before) > 2 || cleared !== null) {
			throw new Error(`reset failed: width=${reset} (expected ${before}), saved=${cleared}`);
		}
		// Collapsed rail (no brand, toggle lives in the footer cluster).
		await s.click('.footer-actions .collapse-btn');
		await s.waitFor(() => document.querySelector('.sidebar.collapsed'), 5000, 'collapsed sidebar');
		await sleep(300);
		await s.shot('sidebar-collapsed');
		await s.click('.mini-footer .mini-btn[title*="Expand"]');
	},
	async sheet(s) {
		await s.unlock();
		await s.openDoc('doc-1');
		await s.click('.more-btn');
		await s.waitFor(() => document.querySelector('.sheet'), 10000, 'action sheet');
		await sleep(400);
		await s.shot('sheet');
	},
	async split(s) {
		await s.unlock();
		await s.openDoc('doc-1');
		const opened = await s.eval(
			`(() => { const b = document.querySelector('.split-btn'); if (!b) return false; b.click(); return true; })()`,
		);
		if (!opened) {
			// Fallback: SvelteKit intercepts same-origin anchor clicks, so a
			// synthetic link is a reliable client-side navigation.
			await s.eval(
				`(() => { const a = document.createElement('a'); a.href = '/split/doc-1'; document.body.appendChild(a); a.click(); a.remove(); return true; })()`,
			);
		}
		await s.waitFor(() => document.querySelector('.split-page'), 20000, 'split page');
		await s.click('.split-add, .split-placeholder');
		await s.waitFor(() => document.querySelector('.picker-item'), 10000, 'page picker');
		await s.eval(
			`(() => { const items = document.querySelectorAll('.picker-item'); (items[1] || items[0]).click(); return true; })()`,
		);
		await s.waitFor(() => document.querySelectorAll('.split-pane').length === 2, 15000, 'two panes');
		await sleep(900);
		await s.shot('split');
	},
	async settingsMobile(s) {
		await s.unlock();
		await openSettings(s);
		await s.shot('settings-top');
		await s.eval(`(() => { const p = document.querySelector('.settings-panel'); p.scrollTop = 0; return true; })()`);
		const m = await panelMetrics(s);
		s.check('settings-mobile-panel-fits', m && m.docW <= m.winW + 1, JSON.stringify(m));
		s.check('settings-mobile-no-clipped-controls', m && m.bad.length === 0, JSON.stringify(m?.bad));
		// Scroll to the very bottom and make sure the footer is reachable.
		await s.eval(`(() => { const p = document.querySelector('.settings-panel'); p.scrollTop = p.scrollHeight; return true; })()`);
		await sleep(300);
		await s.shot('settings-bottom');
		const footerVisible = await s.eval(
			`(() => { const f = document.querySelector('.settings-footer'); if (!f) return false; const r = f.getBoundingClientRect(); return r.top >= 0 && r.bottom <= window.innerHeight + 1; })()`,
		);
		s.check('settings-mobile-footer-reachable', footerVisible, 'footer not visible after scroll');
	},

	async settingsMatrix(s) {
		await s.unlock();
		await openSettings(s);
		await s.shot('settings-initial');
		for (const group of SETTINGS_MATRIX) {
			for (const option of group.options) {
				const res = await clickSeg(s, group.label, option);
				s.check(`seg-${group.key}-${option}`, res === 'ok', `click returned ${res}`);
				if (res !== 'ok') continue;
				await sleep(220);
				const actual = await s.eval(`document.documentElement.getAttribute(${JSON.stringify(group.attr)})`);
				const expected = group.value(option);
				if (group.key === 'theme' && option === 'Auto') {
					// Auto resolves data-theme to the OS preference; the mode lives in
					// localStorage instead.
					const mode = await s.eval(`localStorage.getItem('enclave-theme')`);
					s.check(`attr-${group.key}-${option}`, mode === 'auto', `enclave-theme=${mode}`);
				} else {
					s.check(`attr-${group.key}-${option}`, actual === expected, `expected ${expected} got ${actual}`);
				}
				const m = await panelMetrics(s);
				s.check(`overflow-${group.key}-${option}`, m && m.docW <= m.winW + 1 && m.bad.length === 0, JSON.stringify(m));
				if (group.key === 'uiscale' || group.key === 'font' || group.key === 'density' || group.key === 'pagewidth') {
					await s.shot(`set-${group.key}-${option.toLowerCase()}`);
				}
			}
		}
		// True black is a switch — test on a dark theme so the effect exists.
		await clickSeg(s, 'Theme', 'Dark');
		await toggleSwitch(s, 'True black (OLED)');
		await sleep(250);
		let attr = await s.eval(`document.documentElement.hasAttribute('data-true-black')`);
		s.check('switch-trueblack-on', attr === true, `data-true-black=${attr}`);
		await s.shot('set-trueblack-on');
		await toggleSwitch(s, 'True black (OLED)');
		await sleep(200);
		attr = await s.eval(`document.documentElement.hasAttribute('data-true-black')`);
		s.check('switch-trueblack-off', attr === false, `data-true-black=${attr}`);
		// Accents: 6 presets plus a custom hex through the color input.
		const accentCount = await s.eval(`document.querySelectorAll('.settings-panel .swatch').length`);
		s.check('accent-swatch-count', accentCount === 6, `found ${accentCount}`);
		for (let i = 0; i < 6; i++) {
			await s.eval(`document.querySelectorAll('.settings-panel .swatch')[${i}].click()`);
			await sleep(150);
			const a = await s.eval(`document.documentElement.getAttribute('data-accent')`);
			s.check(`accent-${i}`, a !== 'custom' && a !== null, `data-accent=${a}`);
		}
		await s.eval(`(() => {
      const input = document.querySelector('.settings-panel input[type=color]');
      if (!input) return false;
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
      setter.call(input, '#cc00ff');
      input.dispatchEvent(new Event('input', { bubbles: true }));
      return true;
    })()`);
		await sleep(250);
		const custom = await s.eval(`document.documentElement.getAttribute('data-accent')`);
		const customVar = await s.eval(`getComputedStyle(document.documentElement).getPropertyValue('--color-accent').trim()`);
		s.check('accent-custom', custom === 'custom' && customVar.toLowerCase() === '#cc00ff', `data-accent=${custom} var=${customVar}`);
		await s.shot('set-accent-custom');
		// General + Security toggles.
		await clickSeg(s, 'Home page order', 'Title');
		await sleep(200);
		const savedSort = await s.eval(`JSON.parse(localStorage.getItem('enclave-settings') || '{}').homeSort`);
		s.check('homesort-persist', savedSort === 'title', `homeSort=${savedSort}`);
		await clickSeg(s, 'Home page order', 'Recent');
		const hapticsBefore = await s.eval(`JSON.parse(localStorage.getItem('enclave-settings') || '{}').haptics`);
		await toggleSwitch(s, 'Vibration feedback');
		await sleep(150);
		const hapticsAfter = await s.eval(`JSON.parse(localStorage.getItem('enclave-settings') || '{}').haptics`);
		s.check('switch-haptics', hapticsAfter !== hapticsBefore, `before=${hapticsBefore} after=${hapticsAfter}`);
		await toggleSwitch(s, 'Vibration feedback');
		await toggleSwitch(s, 'Reduce motion');
		await sleep(150);
		attr = await s.eval(`document.documentElement.hasAttribute('data-reduce-motion')`);
		s.check('switch-reduce-motion-on', attr === true, `data-reduce-motion=${attr}`);
		await toggleSwitch(s, 'Reduce motion');
		// Auto-lock: every option persists.
		for (const opt of ['1m', '5m', '15m', '1h', 'Never']) {
			const res = await clickSeg(s, 'Auto-lock after', opt);
			s.check(`autolock-${opt}`, res === 'ok', `click returned ${res}`);
			const v = await s.eval(`JSON.parse(localStorage.getItem('enclave-settings') || '{}').lockAfter`);
			s.check(`autolock-${opt}-persist`, typeof v === 'number', `lockAfter=${v}`);
		}
		await s.shot('settings-lower-sections');
		await s.eval(`(() => { const p = document.querySelector('.settings-panel'); p.scrollTop = p.scrollHeight; return true; })()`);
		await sleep(300);
		await s.shot('settings-bottom-sections');
	},

	async settingsAI(s) {
		await s.unlock();
		await openSettings(s);
		// Open the AI section (may need to scroll inside the panel).
		await s.eval(`(() => {
      const h = [...document.querySelectorAll('.settings-panel h3')].find((x) => x.textContent.includes('AI assistant'));
      h?.scrollIntoView({ block: 'center' });
      return !!h;
    })()`);
		await sleep(250);
		await toggleSwitch(s, 'Enable AI');
		await sleep(350);
		await s.shot('settings-ai-enabled');
		// Point at a dead endpoint and check the error is surfaced.
		await s.fill('.settings-panel input[aria-label="Endpoint URL"]', 'http://127.0.0.1:9');
		await s.eval(`document.querySelector('.settings-panel input[aria-label="Endpoint URL"]').dispatchEvent(new Event('change', { bubbles: true }))`);
		await sleep(300);
		const checkBtn = await s.eval(
			`(() => { const b = [...document.querySelectorAll('.settings-panel button')].find((x) => x.textContent.trim() === 'Check connection'); if (!b) return false; b.click(); return true; })()`,
		);
		s.check('ai-check-button', checkBtn === true, 'Check connection button missing');
		await sleep(1200);
		const status = await s.eval(`document.querySelector('.settings-panel .ai-status')?.textContent?.trim() || ''`);
		s.check('ai-unreachable-status', status.length > 0, `ai-status="${status}"`);
		await s.shot('settings-ai-unreachable');
		// RAG + builtin embeddings toggles persist.
		await toggleSwitch(s, 'Vault-wide answers (RAG)');
		await toggleSwitch(s, 'Offline embeddings');
		await sleep(200);
		const aiSaved = await s.eval(
			`window.__TAURI_INTERNALS__.invoke('get_setting', { key: 'ai' }).then((v) => { try { return JSON.parse(v || '{}'); } catch { return {}; } })`,
			true,
		);
		s.check('ai-settings-persist', typeof aiSaved.rag === 'boolean', JSON.stringify(aiSaved).slice(0, 200));
		await s.shot('settings-ai-toggles');
		await toggleSwitch(s, 'Enable AI');
	},

	async settingsUpdates(s) {
		await s.unlock();
		await openSettings(s);
		await s.eval(`(() => {
      const h = [...document.querySelectorAll('.settings-panel h3')].find((x) => x.textContent.includes('Updates'));
      h?.scrollIntoView({ block: 'center' });
      return true;
    })()`);
		await toggleSwitch(s, 'Allow checking for updates?');
		await sleep(250);
		const enabled = await s.eval(`!document.querySelector('.settings-panel button[disabled]')`);
		await s.shot('settings-updates');
		const clicked = await s.eval(
			`(() => { const b = [...document.querySelectorAll('.settings-panel button')].find((x) => x.textContent.trim() === 'Check'); if (!b || b.disabled) return false; b.click(); return true; })()`,
		);
		s.check('updates-check-click', clicked === true, 'Check button not clickable after enabling updates');
		await sleep(1200);
		await s.shot('settings-update-dialog');
		const dialog = await s.eval(`document.querySelector('.update-dialog') !== null`);
		s.check('updates-dialog-opens', dialog === true, 'no dialog after Check');
		// Approve + download: exercises the no-arg download_update revoke path.
		await s.eval(`(() => { const cb = document.querySelector('.update-dialog input[type=checkbox]'); if (cb && !cb.checked) cb.click(); return true; })()`);
		await sleep(200);
		const dl = await s.eval(
			`(() => { const b = [...document.querySelectorAll('.update-dialog button')].find((x) => x.textContent.includes('Download')); if (!b || b.disabled) return false; b.click(); return true; })()`,
		);
		s.check('updates-download-click', dl === true, 'Download & install not clickable after agreeing');
		await sleep(900);
		const status = await s.eval(`document.querySelector('.update-dialog')?.innerText || ''`);
		s.check('updates-install-status', /installed/i.test(status), `dialog text="${status.replace(/\n+/g, ' ').slice(0, 160)}"`);
		await s.shot('settings-update-installed');
		await s.key('Escape');
		await sleep(300);
	},

	async settingsBackup(s) {
		await s.unlock();
		await openSettings(s);
		await s.eval(`(() => {
      const h = [...document.querySelectorAll('.settings-panel h3')].find((x) => x.textContent.trim() === 'Backup');
      h?.scrollIntoView({ block: 'center' });
      return true;
    })()`);
		await sleep(250);
		const clicked = await s.eval(
			`(() => { const b = [...document.querySelectorAll('.settings-panel button')].find((x) => x.textContent.trim() === 'Back up'); if (!b) return false; b.click(); return true; })()`,
		);
		s.check('backup-click', clicked === true, 'Back up button missing');
		await sleep(600);
		await s.shot('settings-backup');
		const msg = await s.eval(`document.querySelector('.settings-panel .backup-msg')?.textContent?.trim() || ''`);
		s.check('backup-status', msg.length > 0, 'no backup status message');
	},

	async settingsLock(s) {
		await s.unlock();
		await openSettings(s);
		await s.eval(`(() => {
      const h = [...document.querySelectorAll('.settings-panel h3')].find((x) => x.textContent.trim() === 'Security');
      h?.scrollIntoView({ block: 'center' });
      return true;
    })()`);
		await sleep(250);
		const clicked = await s.eval(
			`(() => { const b = [...document.querySelectorAll('.settings-panel button')].find((x) => x.textContent.trim() === 'Lock now'); if (!b) return false; b.click(); return true; })()`,
		);
		s.check('lock-now-click', clicked === true, 'Lock now button missing');
		await s.waitFor(() => document.querySelector('.vault-card'), 10000, 'vault wall after lock');
		await sleep(300);
		await s.shot('settings-locked');
	},

	async settingsScales(s) {
		await s.unlock();
		await openSettings(s);
		// Neutral background so translucency from the Glass preset can't be
		// mistaken for a layout bug.
		await clickSeg(s, 'Background', 'Matte');
		await sleep(200);
		for (const opt of ['Compact', 'Regular', 'Large', 'Xlarge']) {
			await clickSeg(s, 'UI size', opt);
			await sleep(300);
			const geo = await s.eval(`(() => {
        const p = document.querySelector('.settings-panel');
        const h = document.querySelector('.settings-header');
        const r = p.getBoundingClientRect();
        const hr = h ? h.getBoundingClientRect() : null;
        return { top: Math.round(r.top), bottom: Math.round(r.bottom), winH: window.innerHeight, headerTop: hr ? Math.round(hr.top) : null, headerBottom: hr ? Math.round(hr.bottom) : null, closeVisible: (() => { const c = document.querySelector('.settings-close'); if (!c) return false; const cr = c.getBoundingClientRect(); return cr.top >= 0 && cr.bottom <= window.innerHeight; })() };
      })()`);
			s.check(
				`scale-${opt}-panel-on-screen`,
				geo.top >= -1 && geo.bottom <= geo.winH + 1,
				`panel top=${geo.top} bottom=${geo.bottom} viewport=${geo.winH}`,
			);
			s.check(`scale-${opt}-close-visible`, geo.closeVisible, `close button off-screen: ${JSON.stringify(geo)}`);
			await s.shot(`scale-${opt.toLowerCase()}`);
		}
		// Restore regular, then check the AI status row squeezes the button.
		await clickSeg(s, 'UI size', 'Regular');
		await s.eval(`(() => {
      const h = [...document.querySelectorAll('.settings-panel h3')].find((x) => x.textContent.includes('AI assistant'));
      h?.scrollIntoView({ block: 'center' });
      return true;
    })()`);
		await toggleSwitch(s, 'Enable AI');
		await sleep(300);
		await s.fill('.settings-panel input[aria-label="Endpoint URL"]', 'http://127.0.0.1:9');
		await s.eval(`document.querySelector('.settings-panel input[aria-label="Endpoint URL"]').dispatchEvent(new Event('change', { bubbles: true }))`);
		await sleep(200);
		await s.eval(
			`(() => { const b = [...document.querySelectorAll('.settings-panel button')].find((x) => x.textContent.trim() === 'Check connection'); if (b) b.click(); return true; })()`,
		);
		await sleep(1000);
		const btn = await s.eval(`(() => {
      const b = [...document.querySelectorAll('.settings-panel button')].find((x) => (x.textContent || '').includes('Check connection'));
      if (!b) return null;
      const r = b.getBoundingClientRect();
      return { w: Math.round(r.width), h: Math.round(r.height), scrollW: b.scrollWidth, clientW: b.clientWidth, scrollH: b.scrollHeight, clientH: b.clientHeight, text: (b.innerText || '').split(String.fromCharCode(10)).join(' '), rowW: Math.round(b.parentElement.getBoundingClientRect().width) };
    })()`);
		s.check('ai-check-btn-not-clipped', btn && btn.scrollW <= btn.clientW + 1 && btn.scrollH <= btn.clientH + 1, JSON.stringify(btn));
		await s.shot('ai-button-geometry');
	},

	async auditPages(s) {
		await s.unlock();
		await domAudit(s, 'home');
		if (s.profile.mobile) {
			await s.click('.mobile-topbar .topbar-btn');
			if (await s.eval(`!!document.querySelector('.sidebar.open')`)) await domAudit(s, 'drawer');
			await s.key('Escape');
		}
		await s.openDoc('doc-1');
		await domAudit(s, 'doc');
		// Whiteboard view of the same page.
		await s.clickText('Whiteboard');
		await s.waitFor(() => document.querySelector('canvas'), 15000, 'whiteboard canvas');
		await sleep(700);
		await domAudit(s, 'whiteboard');
		await s.clickText('Paper view').catch(() => {});
		await s.keyWithCtrl('k', 'KeyK', 75);
		await s.waitFor(() => document.querySelector('.command-palette'), 10000, 'command palette');
		await sleep(300);
		await domAudit(s, 'palette');
		await s.key('Escape');
		await s.eval(`(() => { const a = document.createElement('a'); a.href = '/graph'; document.body.appendChild(a); a.click(); a.remove(); return true; })()`);
		await s.waitFor(() => document.querySelector('canvas'), 20000, 'graph canvas');
		await sleep(900);
		await domAudit(s, 'graph');
		await s.eval(`(() => { const a = document.createElement('a'); a.href = '/'; document.body.appendChild(a); a.click(); a.remove(); return true; })()`);
		await sleep(600);
		await openSettings(s);
		await domAudit(s, 'settings');
	},

	async appearanceMatrix(s) {
		await s.unlock();
		await openSettings(s);
		const combos = [
			...['Auto', 'Light', 'Dark'].flatMap((m) => ['Matte', 'Soft glow', 'Glass'].map((b) => ({ m, b, tb: false }))),
			{ m: 'Dark', b: 'Matte', tb: true },
			{ m: 'Dark', b: 'Glass', tb: true },
		];
		for (const { m, b, tb } of combos) {
			await clickSeg(s, 'Theme', m);
			await clickSeg(s, 'Background', b);
			await setSwitch(s, 'True black (OLED)', tb);
			await sleep(200);
			await s.shot(`appearance-${m.toLowerCase()}-${b.replace(' ', '')}${tb ? '-oled' : ''}-settings`);
			await s.eval(`document.querySelector('.settings-close')?.click()`);
			await sleep(350);
			await s.shot(`appearance-${m.toLowerCase()}-${b.replace(' ', '')}${tb ? '-oled' : ''}-home`);
			await openSettings(s);
		}
	},

	async flows(s) {
		await s.unlock();
		// Create a page from the sidebar button.
		const created = await s.eval(
			`(() => { const b = [...document.querySelectorAll('button')].find((x) => (x.title || '').startsWith('New page')); if (!b) return false; b.click(); return true; })()`,
		);
		s.check('flow-create-click', created === true, 'New page button missing');
		await s.waitFor(() => document.querySelector('.document-page'), 15000, 'new document page');
		await sleep(600);
		const newTitle = await s.eval(`document.querySelector('.doc-title-input, input.page-title, [contenteditable]')?.textContent || document.querySelector('.document-page')?.innerText?.slice(0, 40)`);
		await s.shot('flow-created');
		// Go home and search the palette.
		await s.eval(`(() => { const a = document.createElement('a'); a.href = '/'; document.body.appendChild(a); a.click(); a.remove(); return true; })()`);
		await sleep(500);
		await s.keyWithCtrl('k', 'KeyK', 75);
		await s.waitFor(() => document.querySelector('.command-palette'), 10000, 'palette');
		await s.fill('.command-palette input', 'roadmap');
		await sleep(2000);
		const hits = await s.eval(`document.querySelectorAll('.command-palette .palette-item').length`);
		const paletteState = await s.eval(`document.querySelector('.command-palette .palette-empty')?.textContent?.trim() || ''`);
		s.check('flow-palette-search', hits >= 1 || paletteState === 'No results found', `results=${hits} state="${paletteState}"`);
		await s.shot('flow-palette-search');
		await s.key('Escape');
		await sleep(300);
		// Open the first doc, delete via action sheet, undo from the snackbar.
		await s.openDoc('doc-1');
		await s.click('.more-btn');
		await s.waitFor(() => document.querySelector('.sheet'), 10000, 'action sheet');
		await s.clickText('Delete page');
		await sleep(800);
		const snack = await s.eval(`document.querySelector('.snackbar, .toast')?.innerText || ''`);
		s.check('flow-delete-snackbar', /trash/i.test(snack), `snackbar="${snack}"`);
		await s.shot('flow-deleted-snackbar');
		const undone = await s.eval(
			`(() => { const b = [...document.querySelectorAll('.snack-undo, .toast-undo, .snackbar button, .toast button')].find((x) => /undo/i.test(x.textContent)); if (!b) return false; b.click(); return true; })()`,
		);
		s.check('flow-undo-click', undone === true, 'Undo button missing');
		await sleep(500);
		await s.shot('flow-undone');
	},

	async overlaysXlarge(s) {
		await s.unlock();
		// Force Xlarge through the settings UI, then close the panel.
		await openSettings(s);
		await clickSeg(s, 'UI size', 'Xlarge');
		await sleep(300);
		await s.eval(`document.querySelector('.settings-close')?.click()`);
		await sleep(350);
		// Action sheet on a doc page.
		await s.openDoc('doc-1');
		await s.click('.more-btn');
		await s.waitFor(() => document.querySelector('.sheet'), 10000, 'action sheet');
		await sleep(400);
		const sheet = await s.eval(`(() => {
      const sh = document.querySelector('.sheet');
      if (!sh) return null;
      const r = sh.getBoundingClientRect();
      const off = [...sh.querySelectorAll('.sheet-item')].filter((el) => { const b = el.getBoundingClientRect(); return b.top < 0 || b.bottom > window.innerHeight; }).map((el) => el.innerText.trim());
      return { top: Math.round(r.top), bottom: Math.round(r.bottom), winH: window.innerHeight, offscreenItems: off };
    })()`);
		s.check('xlarge-sheet-fits', sheet && sheet.top >= -1 && sheet.bottom <= sheet.winH + 1, JSON.stringify(sheet));
	 s.check('xlarge-sheet-items-visible', sheet && sheet.offscreenItems.length === 0, `offscreen: ${JSON.stringify(sheet?.offscreenItems)}`);
		await s.shot('xlarge-sheet');
		await s.key('Escape');
		await sleep(300);
		// Command palette.
		await s.keyWithCtrl('k', 'KeyK', 75);
		await s.waitFor(() => document.querySelector('.command-palette'), 10000, 'palette');
		await sleep(300);
		const pal = await s.eval(`(() => { const p = document.querySelector('.command-palette'); const r = p.getBoundingClientRect(); return { top: Math.round(r.top), bottom: Math.round(r.bottom), winH: window.innerHeight }; })()`);
		s.check('xlarge-palette-fits', pal.top >= -1 && pal.bottom <= pal.winH + 1, JSON.stringify(pal));
		await s.shot('xlarge-palette');
		await s.key('Escape');
		await sleep(300);
		// Settings sheet itself (reopen).
		await openSettings(s);
		const geo = await s.eval(`(() => { const p = document.querySelector('.settings-panel'); const r = p.getBoundingClientRect(); const c = document.querySelector('.settings-close'); const cr = c.getBoundingClientRect(); return { top: Math.round(r.top), bottom: Math.round(r.bottom), winH: window.innerHeight, closeTop: Math.round(cr.top), closeBottom: Math.round(cr.bottom) }; })()`);
		s.check('xlarge-settings-close-reachable', geo.closeTop >= 0 && geo.closeBottom <= geo.winH + 1, JSON.stringify(geo));
	},

	async vaultFlow(s) {
		// Fresh vault: welcome → create password → recovery phrase → shell.
		await s.boot(`${s.base}/?fresh`);
		await s.waitFor(() => document.querySelector('.vault-features'), 25000, 'welcome card');
		await s.fill('#password', 'abc');
		await s.fill('#confirm', 'abc');
		await sleep(200);
		const disabledShort = await s.eval(
			`(() => { const b = [...document.querySelectorAll('button')].find((x) => x.textContent.includes('Create vault')); return b ? b.disabled : null; })()`,
		);
		s.check('vault-short-password-rejected', disabledShort === true, `disabled=${disabledShort}`);
		await s.fill('#password', '1234567');
		await s.fill('#confirm', '1234567');
		await sleep(200);
		const disabledSeven = await s.eval(
			`(() => { const b = [...document.querySelectorAll('button')].find((x) => x.textContent.includes('Create vault')); return b ? b.disabled : null; })()`,
		);
		s.check('vault-7char-password-rejected', disabledSeven === true, `disabled=${disabledSeven}`);
		await s.shot('vault-welcome');
		await s.fill('#password', 'hunter2secret');
		await s.fill('#confirm', 'hunter2secret');
		await sleep(200);
		const enabled = await s.eval(
			`(() => { const b = [...document.querySelectorAll('button')].find((x) => x.textContent.includes('Create vault')); return b ? !b.disabled : null; })()`,
		);
		s.check('vault-valid-password-accepted', enabled === true, `enabled=${enabled}`);
		await s.clickText('Create vault').catch(() => {});
		await s.waitFor(() => document.querySelector('.seed-box'), 90000, 'recovery phrase');
		const wordCount = await s.eval(`document.querySelectorAll('.seed-box .seed-word').length`);
		s.check('vault-seed-12-words', wordCount === 12, `words=${wordCount}`);
		await s.shot('vault-seed');
		await s.clickText("I've saved my recovery phrase");
		await s.waitFor(() => document.querySelector('.sidebar'), 25000, 'app shell after create');
		// Wrong seed phrase on the unlock screen surfaces an inline error.
		await s.boot(`${s.base}/`);
		await s.waitFor(() => document.querySelector('.seed-input'), 25000, 'unlock form');
		await s.fill('.seed-input', 'not a real seed phrase at all just words here okay then');
		await s.clickText('Unlock');
		await sleep(600);
		const err = await s.eval(`document.querySelector('.vault-error')?.textContent?.trim() || ''`);
		s.check('vault-wrong-phrase-error', /invalid seed/i.test(err), `error="${err}"`);
		await s.shot('vault-wrong-phrase');
	},

	async graphPan(s) {
		await s.unlock();
		await s.click('a[href="/graph"]');
		await s.waitFor(() => document.querySelector('canvas'), 20000, 'graph canvas');
		await sleep(1200);
		// Drag from the canvas centre → pan, never navigate.
		const c = await s.eval(`(() => { const r = document.querySelector('.graph-canvas-wrap canvas').getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()`);
		await s.cdp.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', clickCount: 1 });
		for (let i = 1; i <= 6; i++) {
			await s.cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x + i * 20, y: c.y + i * 6, button: 'left' });
			await sleep(25);
		}
		await s.cdp.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x + 120, y: c.y + 36, button: 'left', clickCount: 1 });
		await sleep(300);
		const path = await s.eval(`location.pathname`);
		s.check('graph-pan-no-navigate', path === '/graph', `navigated to ${path}`);
		// Wheel zoom at the centre must not throw or navigate.
		await s.cdp.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: c.x, y: c.y, deltaX: 0, deltaY: -400 });
		await sleep(300);
		await s.cdp.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: c.x, y: c.y, deltaX: 0, deltaY: 800 });
		await sleep(300);
		s.check('graph-zoom-no-navigate', (await s.eval(`location.pathname`)) === '/graph', 'navigated during zoom');
		await s.shot('graph-pan-zoom');
	},

	async capture(s) {
		await s.boot(`${s.base}/capture`);
		await s.waitFor(() => document.querySelector('.note'), 15000, 'capture composer');
		await s.shot('capture');
	},
	async widget(s) {
		await s.boot(`${s.base}/widget`);
		await s.waitFor(() => document.querySelector('.w-list, .w-hint'), 15000, 'widget');
		await sleep(400);
		await s.shot('widget');
	},
};

function defaultTour(profile) {
	return profile.mobile
		? ['welcome', 'unlock', 'home', 'drawer', 'doc', 'sheet', 'palette', 'whiteboard', 'settingsMobile', 'capture', 'widget']
		: ['welcome', 'unlock', 'home', 'sidebar', 'doc', 'whiteboard', 'graph', 'settings', 'palette', 'split', 'capture', 'widget'];
}

function scenariosFor(profile, name) {
	const names = name.split(',').map((s) => s.trim()).filter(Boolean);
	const out = [];
	for (const n of names) {
		if (n === 'all') out.push(...defaultTour(profile));
		else if (SCENARIOS[n]) out.push(n);
		else throw new Error(`unknown scenario: ${n} (have: ${Object.keys(SCENARIOS).join(', ')}, all)`);
	}
	return out.length > 0 ? out : defaultTour(profile);
}

// ── Run ──────────────────────────────────────────────────────────────────────
async function main() {
	const args = parseArgs(process.argv.slice(2));
	if (args.help || !args.url) {
		console.log(`Enclave UI screenshot engine

  node shot.mjs --url <frontend-url> [--profiles desktop,phone] [--scenario all] [--theme dark]
                [--setting uiScale=compact] [--out DIR] [--chrome PATH] [--verbose]

Profiles: ${Object.keys(PROFILES).join(', ')}
Scenarios: ${Object.keys(SCENARIOS).join(', ')}, all
Output goes OUTSIDE the repo (default ${join(HERE, 'shots')}).`);
		process.exit(args.help ? 0 : 2);
	}

	const chromePath = findChrome(args.chrome);
	const profileNames = args.profiles.split(',').map((p) => p.trim()).filter(Boolean);
	for (const p of profileNames) if (!PROFILES[p]) throw new Error(`unknown profile: ${p}`);

	await mkdir(args.out, { recursive: true });
	const chrome = spawn(
		chromePath,
		[
			'--headless=new',
			'--no-sandbox',
			'--disable-gpu',
			'--hide-scrollbars',
			'--remote-debugging-port=0',
			'--window-size=1440,900',
			'--no-first-run',
			'--no-default-browser-check',
			'about:blank',
		],
		{ stdio: ['ignore', 'ignore', 'pipe'] },
	);
	// Chromium prints the browser endpoint (port 0 = ephemeral). We need a page
	// target, so parse the port and ask /json/list for the page websocket.
	const devtoolsPort = await new Promise((resolve, reject) => {
		let buf = '';
		const timer = setTimeout(() => reject(new Error('Chromium did not report a DevTools endpoint')), 20000);
		chrome.stderr.on('data', (d) => {
			buf += d.toString();
			const m = /ws:\/\/127\.0\.0\.1:(\d+)\//.exec(buf);
			if (m) {
				clearTimeout(timer);
				resolve(Number(m[1]));
			}
		});
		chrome.on('exit', (code) => reject(new Error(`Chromium exited early (${code})`)));
	});

	let pageWsUrl;
	for (let i = 0; i < 50 && !pageWsUrl; i++) {
		try {
			const list = await (await fetch(`http://127.0.0.1:${devtoolsPort}/json/list`)).json();
			pageWsUrl = list.find((t) => t.type === 'page')?.webSocketDebuggerUrl;
		} catch {
			/* not up yet */
		}
		if (!pageWsUrl) await sleep(200);
	}
	if (!pageWsUrl) throw new Error('no page target on the Chromium DevTools endpoint');

	const ws = new WebSocket(pageWsUrl);
	await new Promise((res, rej) => {
		ws.onopen = res;
		ws.onerror = () => rej(new Error('DevTools websocket failed'));
	});
	const cdp = new CDP(ws);

	const manifest = { url: args.url, theme: args.theme, startedAt: new Date().toISOString(), runs: [], issues: [] };
	const allIssues = [];
	try {
		await cdp.send('Page.enable');
		await cdp.send('Runtime.enable');
		await cdp.send('Emulation.setEmulatedMedia', {
			features: [{ name: 'prefers-color-scheme', value: args.theme === 'light' ? 'light' : 'dark' }],
		});
		await cdp.send('Page.addScriptToEvaluateOnNewDocument', { source: stubSource() });
		const seed = settingsSeedSource(args.settings);
		if (seed) await cdp.send('Page.addScriptToEvaluateOnNewDocument', { source: seed });
		if (args.verbose) {
			cdp.listeners.set('Runtime.consoleAPICalled', [
				(p) => {
					if (p.type === 'error') console.log('  [console.error]', p.args.map((a) => a.value ?? a.description).join(' '));
				},
			]);
		}

		for (const profileName of profileNames) {
			const profile = PROFILES[profileName];
			console.log(`${profileName}  ${profile.width}x${profile.height}${profile.mobile ? ' (mobile)' : ''}`);
			await cdp.send('Emulation.setDeviceMetricsOverride', {
				width: profile.width,
				height: profile.height,
				deviceScaleFactor: profile.dpr,
				mobile: profile.mobile,
			});
			// Real Android WebView UA — Android-only UI (widget settings, share
			// targets) is hidden otherwise.
			await cdp.send('Emulation.setUserAgentOverride', {
				userAgent: profile.mobile
					? 'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Mobile Safari/537.36'
					: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36',
			});
			const session = new Session(cdp, { url: args.url, out: args.out, profile, profileName, theme: args.theme, verbose: args.verbose });
			for (const scenario of scenariosFor(profile, args.scenario)) {
				try {
					// Hard cap so a wedged CDP call can't stall the whole run.
					await Promise.race([
						SCENARIOS[scenario](session),
						new Promise((_, reject) => setTimeout(() => reject(new Error('scenario timeout')), 150000)),
					]);
					manifest.runs.push({ profile: profileName, scenario, ok: true });
					if (session.issues.length) allIssues.push(...session.issues.map((i) => ({ profile: profileName, scenario, ...i })));
					session.issues = [];
				} catch (e) {
					manifest.runs.push({ profile: profileName, scenario, ok: false, error: String(e?.message ?? e) });
					console.error(`  FAIL ${profileName}/${scenario}: ${e?.message ?? e}`);
					try {
						const txt = await session.eval('document.body.innerText.slice(0, 300)');
						console.error(`  page text: ${String(txt).replace(/\n+/g, ' | ')}`);
						await session.shot(`FAIL-${scenario}`);
					} catch { /* page gone */ }
				}
			}
		}
	} finally {
		manifest.issues = allIssues;
		await writeFile(join(args.out, 'manifest.json'), JSON.stringify(manifest, null, 2));
		if (!args.keep) {
			ws.close();
			chrome.kill('SIGKILL');
		}
	}
	const failed = manifest.runs.filter((r) => !r.ok);
	console.log(`\n${manifest.runs.length - failed.length}/${manifest.runs.length} scenarios ok → ${args.out}`);
	if (failed.length) console.error(`${failed.length} scenario(s) failed`);
	if (allIssues.length) console.error(`${allIssues.length} audit finding(s) — see manifest.json/index.html`);
	// CI contract: any failed scenario OR audit finding fails the run.
	if (failed.length || allIssues.length) process.exitCode = 1;
}

main().catch((e) => {
	console.error(e?.stack ?? String(e));
	process.exit(1);
});
