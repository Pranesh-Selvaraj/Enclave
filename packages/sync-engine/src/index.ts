// Enclave Sync Engine — Yjs CRDT documents with encrypted P2P sync.
// Manages Y.Doc instances and provides hooks for the network transport layer.

import * as Y from 'yjs';

export type SendHandler = (docId: string, payload: string) => void;

interface DocEntry {
	doc: Y.Doc;
	unsubscribe: () => void;
}

/**
 * Manages Yjs documents for sync. One Y.Doc per Enclave document.
 * On local changes, calls `onSend` with the encrypted update.
 * On incoming messages, decrypts and applies to the Y.Doc.
 */
export class SyncEngine {
	private docs = new Map<string, DocEntry>();
	private onSend: SendHandler;
	private encrypt: (plain: Uint8Array) => Promise<Uint8Array>;
	private decrypt: (cipher: Uint8Array) => Promise<Uint8Array>;

	constructor(
		onSend: SendHandler,
		crypto: {
			encrypt: (plain: Uint8Array) => Promise<Uint8Array>;
			decrypt: (cipher: Uint8Array) => Promise<Uint8Array>;
		},
	) {
		this.onSend = onSend;
		this.encrypt = crypto.encrypt;
		this.decrypt = crypto.decrypt;
	}

	/** Create or retrieve a Y.Doc for a given document ID. */
	getDoc(docId: string): Y.Doc {
		const existing = this.docs.get(docId);
		if (existing) return existing.doc;

		const doc = new Y.Doc();
		const onUpdate = async (update: Uint8Array, origin: any) => {
			// Don't re-send updates that came from the network
			if (origin === 'remote') return;
			try {
				const encrypted = await this.encrypt(update);
				const payload = btoa(String.fromCharCode(...encrypted));
				this.onSend(docId, payload);
			} catch (e) {
				console.error('Sync encrypt failed:', e);
			}
		};
		doc.on('update', onUpdate);

		this.docs.set(docId, {
			doc,
			unsubscribe: () => doc.off('update', onUpdate),
		});

		return doc;
	}

	/** Handle an incoming encrypted sync message from a peer. */
	async handleIncoming(docId: string, encryptedPayload: string): Promise<void> {
		try {
			const encrypted = Uint8Array.from(atob(encryptedPayload), (c) => c.charCodeAt(0));
			const plain = await this.decrypt(encrypted);
			const doc = this.getDoc(docId);
			Y.applyUpdate(doc, plain, 'remote');
		} catch (e) {
			console.error('Sync decrypt/apply failed:', e);
		}
	}

	/** Destroy a document and stop syncing it. */
	destroyDoc(docId: string): void {
		const entry = this.docs.get(docId);
		if (entry) {
			entry.unsubscribe();
			entry.doc.destroy();
			this.docs.delete(docId);
		}
	}

	/** Destroy all documents and cleanup. */
	destroy(): void {
		for (const docId of this.docs.keys()) {
			this.destroyDoc(docId);
		}
	}

	/** Get the Yjs text type from a document (convenience). */
	getText(docId: string, name = 'content'): Y.Text {
		return this.getDoc(docId).getText(name);
	}

	/** Get the Yjs XML fragment from a document (for TipTap integration). */
	getXmlFragment(docId: string, name = 'default'): Y.XmlFragment {
		return this.getDoc(docId).getXmlFragment(name);
	}
}
