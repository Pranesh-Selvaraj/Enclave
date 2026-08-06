import * as assert from 'node:assert';
import { WebStore, type KV, type KVStore } from '../src/lib/webStore';

/** In-memory KV adapter — exercises the full WebStore logic without IndexedDB. */
class MemKV implements KV {
	private stores: Record<KVStore, Map<string, unknown>> = {
		meta: new Map(),
		docs: new Map(),
		blocks: new Map(),
	};
	async get(store: KVStore, id: string): Promise<unknown> {
		return this.stores[store].get(id);
	}
	async put(store: KVStore, id: string, value: unknown): Promise<void> {
		this.stores[store].set(id, value);
	}
	async del(store: KVStore, id: string): Promise<void> {
		this.stores[store].delete(id);
	}
	async all(store: KVStore): Promise<Array<{ id: string; value: unknown }>> {
		return [...this.stores[store].entries()].map(([id, value]) => ({ id, value }));
	}
	async clear(): Promise<void> {
		for (const s of Object.values(this.stores)) s.clear();
	}
}

const key = Array.from(crypto.getRandomValues(new Uint8Array(32)));
const WRONG_KEY = Array.from(crypto.getRandomValues(new Uint8Array(32)));
const store = new WebStore(new MemKV());

async function main() {
	// ── Vault lifecycle ──
	assert.strictEqual(await store.isVaultInitialized(), false, 'fresh vault not initialized');
	await store.initVault(key);
	assert.strictEqual(await store.isVaultInitialized(), true, 'init marks vault as initialized');
	await assert.rejects(store.unlockVault(WRONG_KEY), /Invalid vault key/, 'wrong key is rejected');
	await store.lockVault();
	await assert.rejects(store.getDocumentList(), /Vault is locked/, 'locked vault rejects reads');
	await store.unlockVault(key);
	assert.strictEqual((await store.getDocumentList()).length, 0, 'unlock with right key works');
	await assert.rejects(store.initVault(key), /already exists/, 'double init rejected');

	// ── Documents + blocks ──
	const a = await store.createDocument('Alpha');
	assert.strictEqual((await store.getDocument(a.id)).title, 'Alpha', 'getDocument round-trips');
	assert.strictEqual(a.is_archived, false, 'new doc not archived');
	assert.strictEqual((await store.getBlocks(a.id)).length, 1, 'create seeds a paragraph block');

	const content = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'hello world' }] }] };
	await store.upsertBlock({ id: `${a.id}-content`, documentId: a.id, blockType: 'doc', content, sortOrder: 0 });
	const blocks = await store.getBlocks(a.id);
	assert.strictEqual(blocks.length, 2, 'upsert adds the content block');
	assert.deepStrictEqual(blocks.find((b) => b.type === 'doc')?.content, content, 'content round-trips');

	// upsert preserves created_at
	const c0 = blocks.find((b) => b.type === 'doc')!.created_at;
	await store.upsertBlock({ id: `${a.id}-content`, documentId: a.id, blockType: 'doc', content, sortOrder: 0 });
	const c1 = (await store.getBlocks(a.id)).find((b) => b.type === 'doc')!.created_at;
	assert.strictEqual(c0, c1, 'upsert preserves created_at');

	// title update bumps updated_at and rev
	const before = (await store.getDocument(a.id)).rev ?? 0;
	const renamed = await store.updateDocumentTitle(a.id, 'Alpha Renamed');
	assert.strictEqual(renamed.title, 'Alpha Renamed', 'title updates');
	assert.ok((renamed.rev ?? 0) > before, 'title update bumps rev');

	// ── Tags ──
	await store.upsertBlock({ id: `${a.id}-tags`, documentId: a.id, blockType: 'tags', content: { tags: ['one', 'two'] }, sortOrder: 2 });
	const tags = await store.getAllTags();
	assert.deepStrictEqual(tags.find((t) => t.doc_id === a.id)?.tags, ['one', 'two'], 'tags parse from the tags block');

	// ── Search ──
	let hits = await store.searchAll('hello');
	assert.ok(hits.some((h) => h.type === 'content' && h.block_content.includes('hello')), 'content search hits');
	hits = await store.searchAll('alpha');
	assert.ok(hits.some((h) => h.type === 'title' && h.doc_id === a.id), 'title search hits');
	assert.deepStrictEqual(await store.searchAll(''), [], 'empty query returns nothing');

	// ── Backlinks + page list ──
	const b = await store.createDocument('Beta');
	await store.upsertBlock({ id: `${b.id}-content`, documentId: b.id, blockType: 'doc', content: { type: 'doc', content: [{ type: 'text', text: 'see [[Alpha Renamed]]' }] }, sortOrder: 0 });
	const bl = await store.getBacklinks('Alpha Renamed');
	assert.ok(bl.some((x) => x.doc_id === b.id), 'text backlink found');
	const pages = await store.getPageList();
	assert.ok(pages.some((p) => p.title === 'Alpha Renamed'), 'page list includes renamed doc');

	// ── Favorite / duplicate / find-or-create ──
	const fav = await store.toggleFavorite(a.id);
	assert.strictEqual(fav.is_favorite, true, 'favorite toggles on');
	const dup = await store.duplicateDocument(a.id);
	assert.strictEqual(dup.title, 'Copy of Alpha Renamed', 'duplicate prefixes title');
	assert.strictEqual(dup.is_favorite, false, 'duplicate does not inherit favorite');
	assert.strictEqual((await store.getBlocks(dup.id)).length, (await store.getBlocks(a.id)).length, 'duplicate copies blocks');
	assert.strictEqual((await store.findOrCreateDocument('Alpha Renamed')).id, a.id, 'find-or-create returns existing');
	const created = await store.findOrCreateDocument('Brand New');
	assert.strictEqual(created.title, 'Brand New', 'find-or-create creates missing');

	// ── Archive / restore / delete ──
	const arch = await store.archiveDocument(b.id);
	assert.strictEqual(arch.is_archived, true, 'archive flags the doc');
	assert.ok(!(await store.getDocumentList()).some((d) => d.id === b.id), 'archived excluded from list');
	assert.ok((await store.getArchivedDocuments()).some((d) => d.id === b.id), 'archived listed in trash');
	assert.ok(!(await store.searchAll('Beta')).some((h) => h.type === 'title'), 'archived docs excluded from search');
	await store.restoreDocument(b.id);
	assert.ok((await store.getDocumentList()).some((d) => d.id === b.id), 'restore brings doc back');
	assert.ok((await store.searchAll('Beta')).some((h) => h.type === 'title'), 'restored doc is searchable again');

	await store.deleteDocument(b.id);
	assert.ok(!(await store.getDocumentList()).some((d) => d.id === b.id), 'delete removes from list');
	assert.strictEqual((await store.getBlocks(b.id)).length, 0, 'delete removes blocks');
	assert.ok((await store.getDocument(b.id)).deleted_at != null, 'delete leaves a tombstone (mirrors core-db)');

	// ── Vault key file + reset ──
	await store.storeVaultKey([1, 2, 3]);
	assert.deepStrictEqual(await store.loadVaultKey(), [1, 2, 3], 'vault key round-trips');
	// The key file is readable while locked — VaultGuard checks for it before unlock.
	await store.lockVault();
	assert.deepStrictEqual(await store.loadVaultKey(), [1, 2, 3], 'key file readable while locked');
	await store.unlockVault(key);
	await store.resetVault();
	assert.strictEqual(await store.isVaultInitialized(), false, 'reset wipes the vault');

	console.log('webStore: PASS');
}

main().catch((e) => {
	console.error('webStore: FAIL', e);
	process.exit(1);
});
