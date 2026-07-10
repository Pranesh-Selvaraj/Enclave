import type { Document, Block } from '@enclave/ui';
import {
	deriveMasterKey,
	importAesKey,
	encrypt,
	decrypt,
} from '@enclave/crypto';
import { openDB, type IDBPDatabase } from 'idb';

// ── Interface ────────────────────────────────────────────────────────────────

export interface NetworkStatus {
	local_peer_id: string;
	running: boolean;
	port: number;
	peers: { id: string; host: string; port: number; connected: boolean }[];
}

export interface StorageService {
	isVaultInitialized(): Promise<boolean>;
	initVault(key: Uint8Array<ArrayBuffer>): Promise<void>;
	unlockVault(key: Uint8Array<ArrayBuffer>): Promise<void>;
	lockVault(): Promise<void>;
	getDocumentList(): Promise<Document[]>;
	getDocument(id: string): Promise<Document>;
	createDocument(title: string): Promise<Document>;
	deleteDocument(id: string): Promise<void>;
	updateDocumentTitle(id: string, title: string): Promise<Document>;
	getBlocks(documentId: string): Promise<Block[]>;
	upsertBlock(id: string, documentId: string, blockType: string, content: Record<string, unknown>, sortOrder: number): Promise<Block>;
	deleteBlock(id: string): Promise<void>;
	exportMarkdown(filename: string, contents: string): Promise<string>;
	importMarkdown(): Promise<string | null>;
	startNetwork?(): Promise<void>;
	stopNetwork?(): Promise<void>;
	networkStatus?(): Promise<NetworkStatus>;
}

// ── Tauri Backend ────────────────────────────────────────────────────────────

async function tauriBackend(): Promise<StorageService> {
	const { invoke } = await import('@tauri-apps/api/core');

	return {
		isVaultInitialized: () => invoke<boolean>('is_vault_initialized'),
		initVault: (key) => invoke('init_vault', { key: Array.from(key) }),
		unlockVault: (key) => invoke('unlock_vault', { key: Array.from(key) }),
		lockVault: () => invoke('lock_vault'),
		getDocumentList: () => invoke<Document[]>('get_document_list'),
		getDocument: (id) => invoke<Document>('get_document', { id }),
		createDocument: (title) => invoke<Document>('create_document', { title }),
		deleteDocument: (id) => invoke('delete_document', { id }),
		updateDocumentTitle: (id, title) => invoke<Document>('update_document_title', { id, title }),
		getBlocks: (documentId) => invoke<Block[]>('get_blocks', { documentId }),
		upsertBlock: (id, documentId, blockType, content, sortOrder) =>
			invoke<Block>('upsert_block', { id, documentId, blockType, content, sortOrder }),
		deleteBlock: (id) => invoke('delete_block', { id }),
		exportMarkdown: (filename, contents) => invoke<string>('export_markdown', { filename, contents }),
		importMarkdown: async () => {
			// Tauri file dialog — not used in current UI, placeholder
			return null;
		},
		startNetwork: () => invoke('start_network'),
		stopNetwork: () => invoke('stop_network'),
		networkStatus: () => invoke<NetworkStatus>('network_status'),
	};
}

// ── Web Backend (IndexedDB + crypto) ─────────────────────────────────────────

const DB_NAME = 'enclave-web';
const VAULT_VERIFY = 'enclave-vault-ok-v1';

async function webBackend(): Promise<StorageService> {
	const db = await openDB(DB_NAME, 1, {
		upgrade(db) {
			if (!db.objectStoreNames.contains('vault')) db.createObjectStore('vault');
			if (!db.objectStoreNames.contains('documents')) db.createObjectStore('documents', { keyPath: 'id' });
			if (!db.objectStoreNames.contains('blocks')) db.createObjectStore('blocks', { keyPath: 'id' });
		},
	});

	let masterKey: CryptoKey | null = null;

	function now() {
		return new Date().toISOString();
	}

	// ── vault ──

	async function isVaultInitialized() {
		const meta = await db.get('vault', 'meta');
		return meta?.initialized === true;
	}

	async function initVault(key: Uint8Array<ArrayBuffer>) {
		const aesKey = await importAesKey(key);
		const { iv, ciphertext } = await encrypt(VAULT_VERIFY, aesKey);
		await db.put('vault', {
			initialized: true,
			iv: Array.from(iv),
			ciphertext: Array.from(new Uint8Array(ciphertext)),
		}, 'meta');
		masterKey = aesKey;
	}

	async function unlockVault(key: Uint8Array<ArrayBuffer>) {
		const meta = await db.get('vault', 'meta');
		if (!meta?.initialized) throw new Error('Vault not initialized');
		const aesKey = await importAesKey(key);
		// Verify by decrypting the magic value
		try {
			const decrypted = await decrypt(
				new Uint8Array(meta.ciphertext).buffer,
				new Uint8Array(meta.iv) as Uint8Array<ArrayBuffer>,
				aesKey,
			);
			if (decrypted !== VAULT_VERIFY) throw new Error('Invalid vault key');
		} catch {
			throw new Error('Invalid vault key');
		}
		masterKey = aesKey;
	}

	async function lockVault() {
		masterKey = null;
	}

	// ── documents ──

	async function getDocumentList() {
		return db.getAll('documents');
	}

	async function getDocument(id: string) {
		const doc = await db.get('documents', id);
		if (!doc) throw new Error('Document not found');
		return doc;
	}

	async function createDocument(title: string) {
		const doc: Document = {
			id: crypto.randomUUID(),
			title,
			created_at: now(),
			updated_at: now(),
			is_favorite: false,
			is_archived: false,
		};
		await db.put('documents', doc);

		// Create an initial empty paragraph block
		const block: Block = {
			id: crypto.randomUUID(),
			document_id: doc.id,
			type: 'paragraph',
			content: {},
			sort_order: 1,
			created_at: now(),
			updated_at: now(),
		};
		await db.put('blocks', block);

		return doc;
	}

	async function deleteDocument(id: string) {
		await db.delete('documents', id);
		// Cascade delete blocks
		const blocks = await db.getAllFromIndex('blocks', 'document_id', id);
		for (const b of blocks) await db.delete('blocks', b.id);
	}

	async function updateDocumentTitle(id: string, title: string) {
		const doc = await db.get('documents', id);
		if (!doc) throw new Error('Document not found');
		doc.title = title;
		doc.updated_at = now();
		await db.put('documents', doc);
		return doc;
	}

	// ── blocks ──

	async function getBlocks(documentId: string) {
		const all = await db.getAllFromIndex('blocks', 'document_id', documentId);
		return all;
	}

	async function upsertBlock(
		id: string, documentId: string, blockType: string,
		content: Record<string, unknown>, sortOrder: number,
	) {
		const existing = await db.get('blocks', id);
		const block: Block = {
			id,
			document_id: documentId,
			type: blockType,
			content,
			sort_order: sortOrder,
			created_at: existing?.created_at ?? now(),
			updated_at: now(),
		};
		await db.put('blocks', block);
		return block;
	}

	async function deleteBlock(id: string) {
		await db.delete('blocks', id);
	}

	// ── export / import ──

	async function exportMarkdown(_filename: string, contents: string) {
		// ponytail: browser download instead of filesystem write
		const blob = new Blob([contents], { type: 'text/markdown' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = _filename.endsWith('.md') ? _filename : `${_filename}.md`;
		a.click();
		URL.revokeObjectURL(url);
		return a.download;
	}

	async function importMarkdown() {
		return new Promise<string | null>((resolve) => {
			const input = document.createElement('input');
			input.type = 'file';
			input.accept = '.md,.txt,.markdown';
			input.onchange = () => {
				const file = input.files?.[0];
				if (!file) { resolve(null); return; }
				const reader = new FileReader();
				reader.onload = () => resolve(reader.result as string);
				reader.readAsText(file);
			};
			// Resolve null if user cancels (via window focus trick)
			input.oncancel = () => resolve(null);
			input.click();
			// ponytail: cancel detection via focus; add AbortController if needed
			window.addEventListener('focus', () => {
				setTimeout(() => {
					if (!input.files?.length) resolve(null);
				}, 300);
			}, { once: true });
		});
	}

	// ── network (no-op in web) ──

	async function startNetwork() {
		// ponytail: P2P not available in browser; WebRTC could be added later
	}

	async function stopNetwork() {}

	async function networkStatus(): Promise<NetworkStatus> {
		return { local_peer_id: 'web', running: false, port: 0, peers: [] };
	}

	return {
		isVaultInitialized,
		initVault,
		unlockVault,
		lockVault,
		getDocumentList,
		getDocument,
		createDocument,
		deleteDocument,
		updateDocumentTitle,
		getBlocks,
		upsertBlock,
		deleteBlock,
		exportMarkdown,
		importMarkdown,
		startNetwork,
		stopNetwork,
		networkStatus,
	};
}

// ── Auto-detect & singleton ──────────────────────────────────────────────────

let _storage: Promise<StorageService> | null = null;

export function getStorage(): Promise<StorageService> {
	if (!_storage) {
		const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;
		// ponytail: SSR guard — IndexedDB only exists in the browser.
		// On the server, return a no-op proxy; hydration picks up the real
		// backend on the client.
		const isSSR = typeof indexedDB === 'undefined';
		if (isSSR) {
			_storage = Promise.resolve(ssrNoop());
		} else if (isTauri) {
			_storage = tauriBackend();
		} else {
			_storage = webBackend();
		}
	}
	return _storage;
}

// ponytail: return a no-op StorageService for SSR; replaced on client hydration.
function ssrNoop(): StorageService {
	const noop = async () => {};
	const nope = async () => [] as any[];
	const fakeDoc: Document = { id: '', title: '', created_at: '', updated_at: '', is_favorite: false, is_archived: false };
	return {
		isVaultInitialized: async () => false,
		initVault: async () => {},
		unlockVault: async () => {},
		lockVault: async () => {},
		getDocumentList: async () => [],
		getDocument: async () => fakeDoc,
		createDocument: async () => fakeDoc,
		deleteDocument: async () => {},
		updateDocumentTitle: async () => fakeDoc,
		getBlocks: async () => [],
		upsertBlock: async () => ({ id: '', document_id: '', type: '', content: {}, sort_order: 0, created_at: '', updated_at: '' }),
		deleteBlock: async () => {},
		exportMarkdown: async () => '',
		importMarkdown: async () => null,
		startNetwork: async () => {},
		stopNetwork: async () => {},
		networkStatus: async () => ({ local_peer_id: 'ssr', running: false, port: 0, peers: [] }),
	};
}
