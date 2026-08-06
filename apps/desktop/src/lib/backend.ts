// Backend bridge: routes `invoke`/`listen` to Tauri when running in the
// desktop shell, and to the IndexedDB-backed WebStore in a plain browser.
// Frontend code imports invoke/listen from here instead of @tauri-apps/api.

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen } from '@tauri-apps/api/event';
import { webStore } from './webStore.js';

export function isTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/** Invoke a backend command — Tauri IPC in the shell, WebStore in the browser. */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
	if (isTauri()) return tauriInvoke<T>(cmd, args);
	return webInvoke<T>(cmd, args ?? {});
}

/** Subscribe to a Tauri event; no-op (returns an unlisten) in the browser. */
export function listen(
	event: string,
	handler: (event: { payload: any }) => void,
): Promise<() => void> {
	if (isTauri()) return tauriListen(event, handler);
	return Promise.resolve(() => {});
}

// ── Browser dispatch ─────────────────────────────────────────────────────────

const NETWORK_STUB = {
	local_peer_id: '',
	running: false,
	port: 0,
	peers: [],
};

async function webInvoke<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
	const s = webStore;
	switch (cmd) {
		// vault lifecycle
		case 'is_vault_initialized':
			return s.isVaultInitialized() as T;
		case 'init_vault':
			return s.initVault(args.key as number[]) as T;
		case 'unlock_vault':
			return s.unlockVault(args.key as number[]) as T;
		case 'lock_vault':
			return s.lockVault() as T;
		case 'reset_vault':
			return s.resetVault() as T;
		case 'store_vault_key':
			return s.storeVaultKey(args.keyData as number[]) as T;
		case 'load_vault_key':
			return s.loadVaultKey() as T;

		// documents
		case 'get_document_list':
			return s.getDocumentList() as T;
		case 'get_archived_documents':
			return s.getArchivedDocuments() as T;
		case 'get_document':
			return s.getDocument(args.id as string) as T;
		case 'create_document':
			return s.createDocument(args.title as string) as T;
		case 'delete_document':
			return s.deleteDocument(args.id as string) as T;
		case 'archive_document':
			return s.archiveDocument(args.id as string) as T;
		case 'restore_document':
			return s.restoreDocument(args.id as string) as T;
		case 'duplicate_document':
			return s.duplicateDocument(args.id as string) as T;
		case 'toggle_favorite':
			return s.toggleFavorite(args.id as string) as T;
		case 'update_document_title':
			return s.updateDocumentTitle(args.id as string, args.title as string) as T;
		case 'find_or_create_document':
			return s.findOrCreateDocument(args.title as string) as T;

		// blocks
		case 'get_blocks':
			return s.getBlocks(args.documentId as string) as T;
		case 'upsert_block':
			return s.upsertBlock(args as never) as T;

		// search / tags / backlinks / pages
		case 'search_all':
			return s.searchAll(args.query as string) as T;
		case 'get_all_tags':
			return s.getAllTags() as T;
		case 'get_backlinks':
			return s.getBacklinks(args.title as string) as T;
		case 'find_relation_backlinks':
			return s.findRelationBacklinks(args.docId as string) as T;
		case 'get_page_list':
			return s.getPageList() as T;

		// P2P sync / native fs are Tauri-only — fail clearly instead of crashing.
		case 'network_status':
			return NETWORK_STUB as T;
		case 'start_network':
		case 'stop_network':
		case 'write_file':
		case 'import_markdown':
		case 'export_file':
		case 'save_attachment':
		case 'backup_vault':
			throw new Error(`${cmd} is not available in the web build (Tauri only)`);

		default:
			throw new Error(`Unknown backend command: ${cmd}`);
	}
}
