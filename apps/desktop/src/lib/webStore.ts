// Web storage backend: IndexedDB + WebCrypto AES-256-GCM.
//
// Mirrors the core-db command surface (Document/Block CRUD, vault lifecycle,
// tags, search, backlinks) so the static SPA runs without the Rust backend.
// Every record is encrypted at rest with the derived master key; only the
// record id (an IndexedDB key) stays plaintext, the same metadata SQLCipher
// leaks (schema/page counts).
//
// The store is written against a tiny KV interface so the full command
// surface is unit-testable in Node without IndexedDB (see tests/webStore.test.ts).

import { importAesKey, encrypt, decrypt } from '@enclave/crypto';
import type { Document, Block } from '@enclave/ui';

// ── KV interface ─────────────────────────────────────────────────────────────

export type KVStore = 'meta' | 'docs' | 'blocks';

export interface KV {
	get(store: KVStore, id: string): Promise<unknown>;
	put(store: KVStore, id: string, value: unknown): Promise<void>;
	del(store: KVStore, id: string): Promise<void>;
	all(store: KVStore): Promise<Array<{ id: string; value: unknown }>>;
	clear(): Promise<void>;
}

export interface SearchResult {
	doc_id: string;
	doc_title: string;
	block_content: string;
	/** "title" | "content" — mirrors core-db's SearchResult (r#type → "type"). */
	type: string;
}

export interface Backlink {
	doc_id: string;
	doc_title: string;
	block_content: string;
}

// ── Vault markers ────────────────────────────────────────────────────────────

const MARKER_PLAINTEXT = 'enclave-vault-v1';
const K_MARKER = 'marker'; // encrypted constant used to verify the unlock key
const K_EXISTS = 'vault_exists';
const K_VAULT_KEY = 'vault_key'; // encrypted seed (frontend encrypts with password)

type EncRecord = { iv: Uint8Array<ArrayBuffer>; ct: ArrayBuffer };

// ── Store ────────────────────────────────────────────────────────────────────

export class WebStore {
	private kv: KV;
	private key: CryptoKey | null = null;

	constructor(kv: KV) {
		this.kv = kv;
	}

	private requireKey(): CryptoKey {
		if (!this.key) throw new Error('Vault is locked');
		return this.key;
	}

	private now(): string {
		return new Date().toISOString();
	}

	private async enc(obj: unknown): Promise<EncRecord> {
		const { iv, ciphertext } = await encrypt(JSON.stringify(obj), this.requireKey());
		return { iv, ct: ciphertext };
	}

	private async dec(rec: EncRecord): Promise<any> {
		return JSON.parse(await decrypt(rec.ct, rec.iv, this.requireKey()));
	}

	// ── Vault lifecycle ──

	async isVaultInitialized(): Promise<boolean> {
		return (await this.kv.get('meta', K_EXISTS)) != null;
	}

	async initVault(keyBytes: number[]): Promise<void> {
		if (await this.isVaultInitialized()) throw new Error('Vault already exists');
		this.key = await importAesKey(Uint8Array.from(keyBytes) as Uint8Array<ArrayBuffer>);
		const { iv, ciphertext } = await encrypt(MARKER_PLAINTEXT, this.key);
		await this.kv.put('meta', K_MARKER, { iv, ct: ciphertext });
		await this.kv.put('meta', K_EXISTS, 1);
	}

	async unlockVault(keyBytes: number[]): Promise<void> {
		const marker = (await this.kv.get('meta', K_MARKER)) as EncRecord | undefined;
		if (!marker) throw new Error('Vault is not initialized');
		const key = await importAesKey(Uint8Array.from(keyBytes) as Uint8Array<ArrayBuffer>);
		let ok = false;
		try {
			ok = (await decrypt(marker.ct, marker.iv, key)) === MARKER_PLAINTEXT;
		} catch {
			// AES-GCM auth failure on the wrong key
		}
		if (!ok) throw new Error('Invalid vault key');
		this.key = key;
	}

	async lockVault(): Promise<void> {
		this.key = null;
	}

	async resetVault(): Promise<void> {
		this.key = null;
		await this.kv.clear();
	}

	async storeVaultKey(keyData: number[]): Promise<void> {
		await this.kv.put('meta', K_VAULT_KEY, keyData);
	}

	async loadVaultKey(): Promise<number[]> {
		const v = await this.kv.get('meta', K_VAULT_KEY);
		if (v == null) throw new Error('No password set');
		return v as number[];
	}

	// ── Documents ──

	private async allDocs(): Promise<Document[]> {
		this.requireKey(); // empty vault must still report locked (mirrors with_db)
		const rows = await this.kv.all('docs');
		return Promise.all(rows.map((r) => this.dec(r.value as EncRecord)));
	}

	private async putDoc(doc: Document): Promise<void> {
		await this.kv.put('docs', doc.id, await this.enc(doc));
	}

	private async getDocRaw(id: string): Promise<Document> {
		this.requireKey();
		const rec = (await this.kv.get('docs', id)) as EncRecord | undefined;
		if (!rec) throw new Error('Document not found');
		return this.dec(rec);
	}

	/** Bump the LWW clock and touch updated_at (mirrors core-db bump_rev). */
	private async bump(doc: Document, updatedAt: string): Promise<Document> {
		doc.rev = (doc.rev ?? 0) + 1;
		doc.updated_at = updatedAt;
		return doc;
	}

	async getDocumentList(): Promise<Document[]> {
		const docs = await this.allDocs();
		return docs
			.filter((d) => !d.is_archived && !d.deleted_at)
			.sort((a, b) => b.updated_at.localeCompare(a.updated_at));
	}

	async getArchivedDocuments(): Promise<Document[]> {
		const docs = await this.allDocs();
		return docs
			.filter((d) => d.is_archived && !d.deleted_at)
			.sort((a, b) => b.updated_at.localeCompare(a.updated_at));
	}

	async getDocument(id: string): Promise<Document> {
		return this.getDocRaw(id);
	}

	async createDocument(title: string): Promise<Document> {
		const now = this.now();
		const doc: Document = {
			id: crypto.randomUUID(),
			title,
			created_at: now,
			updated_at: now,
			is_favorite: false,
			is_archived: false,
			rev: 1,
			deleted_at: null,
		};
		await this.putDoc(doc);
		const block: Block = {
			id: crypto.randomUUID(),
			document_id: doc.id,
			type: 'paragraph',
			content: {},
			sort_order: 1,
			created_at: now,
			updated_at: now,
		};
		await this.putBlock(block);
		return doc;
	}

	async deleteDocument(id: string): Promise<void> {
		const now = this.now();
		const doc = await this.getDocRaw(id);
		doc.deleted_at = now;
		await this.putDoc(await this.bump(doc, now));
		for (const b of await this.allBlocks()) {
			if (b.document_id === id) await this.kv.del('blocks', b.id);
		}
	}

	async archiveDocument(id: string): Promise<Document> {
		const doc = await this.getDocRaw(id);
		doc.is_archived = true;
		await this.putDoc(await this.bump(doc, this.now()));
		return doc;
	}

	async restoreDocument(id: string): Promise<Document> {
		const doc = await this.getDocRaw(id);
		doc.is_archived = false;
		await this.putDoc(await this.bump(doc, this.now()));
		return doc;
	}

	async toggleFavorite(id: string): Promise<Document> {
		const doc = await this.getDocRaw(id);
		doc.is_favorite = !doc.is_favorite;
		await this.putDoc(await this.bump(doc, this.now()));
		return doc;
	}

	async updateDocumentTitle(id: string, title: string): Promise<Document> {
		const doc = await this.getDocRaw(id);
		doc.title = title;
		await this.putDoc(await this.bump(doc, this.now()));
		return doc;
	}

	async duplicateDocument(id: string): Promise<Document> {
		const original = await this.getDocRaw(id);
		const now = this.now();
		const doc: Document = {
			id: crypto.randomUUID(),
			title: `Copy of ${original.title}`,
			created_at: now,
			updated_at: now,
			is_favorite: false,
			is_archived: false,
			rev: 1,
			deleted_at: null,
		};
		await this.putDoc(doc);
		for (const b of (await this.allBlocks()).filter((x) => x.document_id === id)) {
			await this.putBlock({ ...b, id: crypto.randomUUID(), document_id: doc.id, created_at: now, updated_at: now });
		}
		return doc;
	}

	async findOrCreateDocument(title: string): Promise<Document> {
		const existing = (await this.allDocs()).find(
			(d) => d.title === title && !d.is_archived && !d.deleted_at,
		);
		if (existing) return existing;
		return this.createDocument(title);
	}

	// ── Blocks ──

	private async allBlocks(): Promise<Block[]> {
		this.requireKey();
		const rows = await this.kv.all('blocks');
		return Promise.all(rows.map((r) => this.dec(r.value as EncRecord)));
	}

	private async putBlock(block: Block): Promise<void> {
		await this.kv.put('blocks', block.id, await this.enc(block));
	}

	async getBlocks(documentId: string): Promise<Block[]> {
		const blocks = await this.allBlocks();
		return blocks
			.filter((b) => b.document_id === documentId)
			.sort((a, b) => a.sort_order - b.sort_order);
	}

	async upsertBlock(args: {
		id: string;
		documentId: string;
		blockType: string;
		content: unknown;
		sortOrder: number;
	}): Promise<Block> {
		const now = this.now();
		const existing = (await this.kv.get('blocks', args.id)) as EncRecord | undefined;
		const created_at = existing ? (await this.dec(existing)).created_at : now;
		const block: Block = {
			id: args.id,
			document_id: args.documentId,
			type: args.blockType,
			content: args.content as Record<string, unknown>,
			sort_order: args.sortOrder,
			created_at,
			updated_at: now,
		};
		await this.putBlock(block);
		// Touching a block bumps the owning document's LWW clock (mirrors core-db).
		const docRec = (await this.kv.get('docs', args.documentId)) as EncRecord | undefined;
		if (docRec) {
			const doc = await this.dec(docRec);
			await this.putDoc(await this.bump(doc, now));
		}
		return block;
	}

	// ── Search / tags / backlinks / pages ──

	private extractText(v: unknown, out: string[]): void {
		if (typeof v === 'string') out.push(v);
		else if (Array.isArray(v)) for (const c of v) this.extractText(c, out);
		else if (v && typeof v === 'object') {
			for (const [k, val] of Object.entries(v)) {
				if (k === 'type' || k === 'attrs') continue; // same skip-list as core-db
				this.extractText(val, out);
			}
		}
	}

	// ponytail: substring match, not FTS5 token/prefix ranking. Fine for a
	// personal KB; swap in a real index (or an in-browser FTS lib) if vaults
	// get large enough that search latency matters.
	async searchAll(query: string): Promise<SearchResult[]> {
		const q = query.trim().toLowerCase();
		if (!q) return [];
		const docs = await this.allDocs();
		const live = docs.filter((d) => !d.is_archived && !d.deleted_at);
		const byId = new Map(live.map((d) => [d.id, d]));
		const out: SearchResult[] = [];
		for (const d of live) {
			if (d.title.toLowerCase().includes(q)) {
				out.push({ doc_id: d.id, doc_title: d.title, block_content: '', type: 'title' });
			}
		}
		for (const b of await this.allBlocks()) {
			if (b.type !== 'doc' || !byId.has(b.document_id)) continue; // prose blocks only
			const parts: string[] = [];
			this.extractText(b.content, parts);
			const text = parts.join(' ');
			if (text.toLowerCase().includes(q)) {
				const d = byId.get(b.document_id)!;
				out.push({ doc_id: d.id, doc_title: d.title, block_content: text, type: 'content' });
			}
		}
		return out.slice(0, 30);
	}

	async getAllTags(): Promise<Array<{ doc_id: string; tags: string[] }>> {
		const live = new Set(
			(await this.allDocs()).filter((d) => !d.is_archived && !d.deleted_at).map((d) => d.id),
		);
		const out: Array<{ doc_id: string; tags: string[] }> = [];
		for (const b of await this.allBlocks()) {
			if (b.type !== 'tags' || !live.has(b.document_id)) continue;
			const tags = (b.content as { tags?: unknown })?.tags;
			out.push({
				doc_id: b.document_id,
				tags: Array.isArray(tags) ? tags.filter((t): t is string => typeof t === 'string') : [],
			});
		}
		return out;
	}

	async getBacklinks(title: string): Promise<Backlink[]> {
		if (!title) return [];
		const docs = await this.allDocs();
		const byId = new Map(docs.map((d) => [d.id, d]));
		const pattern = `[[${title}]]`;
		const hits: Backlink[] = [];
		for (const b of await this.allBlocks()) {
			const json = JSON.stringify(b.content);
			if (json.includes(pattern)) {
				const d = byId.get(b.document_id);
				if (d) hits.push({ doc_id: d.id, doc_title: d.title, block_content: json });
			}
		}
		return hits.sort((a, b) =>
			(byId.get(b.doc_id)?.updated_at ?? '').localeCompare(byId.get(a.doc_id)?.updated_at ?? ''),
		);
	}

	// ponytail: relation backlinks require parsing database blocks, which are
	// Tauri-gated in the editor package anyway. Stub returns [] (the frontend
	// treats backlinks as best-effort); implement when database blocks ship on web.
	async findRelationBacklinks(_docId: string): Promise<Backlink[]> {
		return [];
	}

	async getPageList(): Promise<Array<{ id: string; title: string }>> {
		return (await this.allDocs())
			.filter((d) => !d.is_archived && !d.deleted_at)
			.map((d) => ({ id: d.id, title: d.title }))
			.sort((a, b) => a.title.localeCompare(b.title));
	}
}

// ── IndexedDB adapter (browser) ─────────────────────────────────────────────

let dbPromise: Promise<IDBDatabase> | null = null;

function openDB(): Promise<IDBDatabase> {
	if (!dbPromise) {
		dbPromise = new Promise((resolve, reject) => {
			const req = indexedDB.open('enclave-web', 1);
			req.onupgradeneeded = () => {
				const d = req.result;
				for (const s of ['meta', 'docs', 'blocks'] as const) {
					if (!d.objectStoreNames.contains(s)) d.createObjectStore(s, { keyPath: 'id' });
				}
			};
			req.onsuccess = () => resolve(req.result);
			req.onerror = () => reject(req.error);
		});
	}
	return dbPromise;
}

function idbReq<T>(req: IDBRequest<T>): Promise<T> {
	return new Promise((resolve, reject) => {
		req.onsuccess = () => resolve(req.result);
		req.onerror = () => reject(req.error);
	});
}

export const idbKV: KV = {
	async get(store, id) {
		const d = await openDB();
		return (await idbReq(d.transaction(store, 'readonly').objectStore(store).get(id))) ?? undefined;
	},
	async put(store, id, value) {
		const d = await openDB();
		await idbReq(d.transaction(store, 'readwrite').objectStore(store).put({ id, value }));
	},
	async del(store, id) {
		const d = await openDB();
		await idbReq(d.transaction(store, 'readwrite').objectStore(store).delete(id));
	},
	async all(store) {
		const d = await openDB();
		const rows = (await idbReq(d.transaction(store, 'readonly').objectStore(store).getAll())) as Array<{
			id: string;
			value: unknown;
		}>;
		return rows;
	},
	async clear() {
		const d = await openDB();
		await Promise.all(
			(['meta', 'docs', 'blocks'] as const).map((s) =>
				idbReq(d.transaction(s, 'readwrite').objectStore(s).clear()),
			),
		);
	},
};

/** Browser singleton wired to IndexedDB. */
export const webStore = new WebStore(idbKV);
