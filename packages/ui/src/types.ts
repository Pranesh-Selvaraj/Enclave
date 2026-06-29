/** A document (page) as returned by the Rust backend. */
export interface Document {
	id: string;
	title: string;
	created_at: string;
	updated_at: string;
	is_favorite: boolean;
	is_archived: boolean;
}

/** A content block within a document. */
export interface Block {
	id: string;
	document_id: string;
	type: string;
	content: Record<string, unknown>;
	sort_order: number;
	created_at: string;
	updated_at: string;
}
