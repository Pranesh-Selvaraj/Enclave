<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { invoke } from '$lib/backend.js';
	import { TipTapEditor, SlashMenu, BubbleMenu, PageLinkMenu, MentionMenu, TocPanel, DragHandleMenu, EditorContextMenu, TableMenu } from '@enclave/editor';
	import type { Document, Block } from '@enclave/ui';
	import { htmlToMarkdown, markdownToJson } from '@enclave/editor';
	import { EmojiPicker } from '@enclave/ui';
	import { exportMarkdownDialog, exportHtmlDialog } from '$lib/importExport.js';
	import { saveWithRetry } from '$lib/saveRetry.js';
	import ActionSheet from '$lib/ActionSheet.svelte';
	import { Icon } from '@enclave/ui';
	import Whiteboard from '$lib/Whiteboard.svelte';
	import { loadAISettings, chatStream, embedText, embedLocal, type ChatMessage, type AISettings } from '$lib/ai.js';

	const COVERS = ['grad-0', 'grad-1', 'grad-2', 'grad-3', 'grad-4', 'grad-5'];

	let document = $state<Document | null>(null);
	let documentTitle = $state('');
	let editor = $state(undefined as any);
	let loading = $state(true);
	let backlinks = $state<Array<{ doc_id: string; doc_title: string; block_content: string }>>([]);
	let editorContent = $state<object | undefined>(undefined);
	let pageList = $state<{ id: string; title: string }[]>([]);
	let mode = $state<'paper' | 'whiteboard'>('paper');	/** True once the page has a whiteboard block — only then is the whiteboard a real part of the page. */
	let hasWhiteboard = $state(false);
	// Markdown source view: the page's content as editable markdown text.
	let sourceOpen = $state(false);
	let sourceText = $state('');
	let sourceDirty = $state(false);
	let tags = $state<string[]>([]);
	let tagInput = $state('');
	let comments = $state<{ id: string; text: string; at: string }[]>([]);
	let commentInput = $state('');
	let aiEnabled = $state(false);
	let aiOpen = $state(false);
	let aiMessages = $state<ChatMessage[]>([]);
	let aiQuestion = $state('');
	let aiBusy = $state(false);
	let aiError = $state('');
	let aiAbort: (() => void) | undefined;
	let aiSources = $state<{ id: string; title: string }[]>([]);
	let lastEmbeddedText = '';
	let icon = $state('');
	let cover = $state('');
	let fullWidth = $state(false);
	let metaOpen = $state(false);
	let exportOpen = $state(false);
	let infoOpen = $state(false);
	let toast = $state<string | null>(null);
	let toastTimer: ReturnType<typeof setTimeout>;	// Save feedback: 'saving' while writing, 'error' if it failed after retries.
	let saveState = $state<'saved' | 'saving' | 'error'>('saved');
	let saveVersion = 0;
	// Mobile action sheet: the topbar's icon cluster collapses into this.
	let sheetOpen = $state(false);
	const sheetItems = $derived<{ icon: string; label: string; danger?: boolean; action: () => void }[]>([
		{ icon: 'star', label: document?.is_favorite ? 'Remove from favorites' : 'Add to favorites', action: toggleFavorite },
		...(aiEnabled ? [{ icon: 'sparkles', label: 'Ask AI', action: () => (aiOpen = true) }] : []),
		{ icon: 'download', label: 'Export Markdown…', action: exportMarkdown },
		{ icon: 'download', label: 'Export HTML…', action: exportHtml },
		{ icon: 'print', label: 'Print (PDF)…', action: printPage },
		{ icon: 'info', label: 'Page info', action: () => (infoOpen = true) },
		{ icon: 'trash', label: 'Delete page', danger: true, action: deleteDocument },
	]);

	const docId = $derived($page.params.id);

	// ── Debounce helpers ──
	let titleSaveTimer: ReturnType<typeof setTimeout>;
	let contentSaveTimer: ReturnType<typeof setTimeout>;
	let tagSaveTimer: ReturnType<typeof setTimeout>;
	let metaSaveTimer: ReturnType<typeof setTimeout>;

	/** Run all pending debounced saves immediately (app close / tab hide). */
	function flushPendingSaves() {
		if (titleSaveTimer) {
			clearTimeout(titleSaveTimer);
			titleSaveTimer = undefined as never;
			void saveTitle();
		}
		if (contentSaveTimer) {
			clearTimeout(contentSaveTimer);
			contentSaveTimer = undefined as never;
			void saveContent();
		}
		if (tagSaveTimer) {
			clearTimeout(tagSaveTimer);
			tagSaveTimer = undefined as never;
			void saveTags();
		}
		if (metaSaveTimer) {
			clearTimeout(metaSaveTimer);
			metaSaveTimer = undefined as never;
			void saveMeta();
		}
	}

	$effect(() => {
		// `document` here is the page's local state — use window.document.
		const onVis = () => {
			if (window.document.visibilityState === 'hidden') flushPendingSaves();
		};
		const onUnload = () => flushPendingSaves();
		window.document.addEventListener('visibilitychange', onVis);
		window.addEventListener('beforeunload', onUnload);
		return () => {
			window.document.removeEventListener('visibilitychange', onVis);
			window.removeEventListener('beforeunload', onUnload);
		};
	});

	function saveTags() {
		try {
			invoke('upsert_block', {
				id: `${docId}-tags`,
				documentId: docId,
				blockType: 'tags',
				content: { tags },
				sortOrder: 2,
			});
		} catch (e) {
			console.error('Failed to save tags:', e);
		}
	}

	function saveComments() {
		try {
			invoke('upsert_block', {
				id: `${docId}-comments`,
				documentId: docId,
				blockType: 'comments',
				content: { comments },
				sortOrder: 4,
			});
		} catch (e) {
			console.error('Failed to save comments:', e);
		}
	}

	function addComment() {
		const text = commentInput.trim();
		if (!text) return;
		comments = [...comments, { id: crypto.randomUUID(), text, at: new Date().toISOString() }];
		commentInput = '';
		saveComments();
	}

	function removeComment(id: string) {
		comments = comments.filter((c) => c.id !== id);
		saveComments();
	}

	function addTag() {
		const t = tagInput.trim().replace(/^#/, '');
		if (t && !tags.includes(t)) tags = [...tags, t];
		tagInput = '';
		clearTimeout(tagSaveTimer);
		tagSaveTimer = setTimeout(saveTags, 500);
	}

	function removeTag(t: string) {
		tags = tags.filter((x) => x !== t);
		clearTimeout(tagSaveTimer);
		tagSaveTimer = setTimeout(saveTags, 500);
	}

	function setMode(m: 'paper' | 'whiteboard') {
		mode = m;
		try { localStorage.setItem(`enclave-mode-${docId}`, m); } catch { /* ignore */ }
	}

	function saveMeta() {
		try {
			invoke('upsert_block', {
				id: `${docId}-meta`,
				documentId: docId,
				blockType: 'meta',
				content: { icon, cover },
				sortOrder: 3,
			});
		} catch (e) {
			console.error('Failed to save page meta:', e);
		}
	}

	function setIcon(next: string) {
		icon = next;
		clearTimeout(metaSaveTimer);
		metaSaveTimer = setTimeout(saveMeta, 300);
	}

	function setCover(next: string) {
		cover = next;
		clearTimeout(metaSaveTimer);
		metaSaveTimer = setTimeout(saveMeta, 300);
	}

	function toggleFullWidth() {
		fullWidth = !fullWidth;
		try { localStorage.setItem(`enclave-fullwidth-${docId}`, String(fullWidth)); } catch { /* ignore */ }
	}

	// ── Markdown source view ──
	async function toggleSource() {
		if (sourceOpen) {
			await applySource();
		} else if (editor) {
			sourceText = htmlToMarkdown(editor.getHTML());
			sourceDirty = false;
		}
		sourceOpen = !sourceOpen;
	}

	async function applySource() {
		if (!sourceDirty || !editor) return;
		sourceDirty = false;
		try {
			const json = await markdownToJson(sourceText);
			// Only touch the doc when the source actually changed.
			if (JSON.stringify(json) !== JSON.stringify(editor.getJSON())) {
				editor.commands.setContent(json);
				handleEditorChange();
			}
		} catch (e) {
			console.error('Failed to parse markdown source:', e);
		}
	}

	async function loadDocument() {
		try {
			document = await invoke<Document>('get_document', { id: docId });
			documentTitle = document.title || '';
		} catch (e) {
			console.error('Failed to load document:', e);
		}
	}

	async function loadBlocks() {
		try {
			const blocks = await invoke<Block[]>('get_blocks', { documentId: docId });
			const tagsBlock = blocks.find(b => b.type === 'tags');
			if (tagsBlock?.content && Array.isArray((tagsBlock.content as any).tags)) {
				tags = ((tagsBlock.content as any).tags as unknown[]).filter(t => typeof t === 'string');
			}
			const metaBlock = blocks.find(b => b.type === 'meta');
			const meta = (metaBlock?.content as { icon?: string; cover?: string } | undefined) ?? {};
			icon = meta.icon ?? '';
			cover = meta.cover ?? '';
			hasWhiteboard = blocks.some(b => b.type === 'whiteboard');
			const commentsBlock = blocks.find(b => b.type === 'comments');
			if (commentsBlock?.content && Array.isArray((commentsBlock.content as any).comments)) {
				comments = ((commentsBlock.content as any).comments as { id: string; text: string; at: string }[])
					.filter((c) => c && typeof c.text === 'string');
			}
			try { fullWidth = localStorage.getItem(`enclave-fullwidth-${docId}`) === 'true'; } catch { fullWidth = false; }
			if (blocks.length > 0) {
				const contentBlock = blocks.find(b => {
					if (typeof b.content === 'object' && b.content !== null) {
						if ((b.content as any).type === 'doc') return true;
						if ((b.content as any).text) return true;
					}
					return false;
				});
				if (contentBlock && (contentBlock.content as any).type === 'doc') {
					editorContent = contentBlock.content as object;
				}
			}
		} catch (e) {
			console.error('Failed to load blocks:', e);
		} finally {
			loading = false;
		}
	}

	async function loadBacklinks() {
		if (!documentTitle) return;
		try {
			const [text, rel] = await Promise.all([
				invoke<typeof backlinks>('get_backlinks', { title: documentTitle }),
				invoke<typeof backlinks>('find_relation_backlinks', { docId }),
			]);
			const seen = new Set<string>();
			backlinks = [...text, ...rel].filter((b) => {
				const k = b.doc_id + '\u0000' + b.block_content;
				if (seen.has(k)) return false;
				seen.add(k);
				return true;
			});
		} catch (e) {
			// Backlinks are non-critical; ignore errors
		}
	}

	async function loadPageList() {
		try {
			pageList = await invoke<{ id: string; title: string }[]>('get_page_list');
		} catch (e) {
			console.error('Failed to load page list:', e);
		}
	}

	async function saveTitle() {
		if (!document) return;
		try {
			document = await invoke<Document>('update_document_title', {
				id: docId,
				title: documentTitle,
			});
		} catch (e) {
			console.error('Failed to save title:', e);
		}
	}

	function debouncedSaveTitle() {
		clearTimeout(titleSaveTimer);
		titleSaveTimer = setTimeout(saveTitle, 500);
	}

	async function saveContent() {
		if (!document || !editor) return;
		// Serialize once, here — not on every keystroke.
		const json = editor.getJSON();
		const expected = ++saveVersion;
		saveState = 'saving';
		const res = await saveWithRetry(
			() => invoke('upsert_block', {
				id: `${docId}-content`,
				documentId: docId,
				blockType: 'doc',
				content: json,
				sortOrder: 0,
			}),
			() => saveVersion,
			expected,
		);
		// A newer save already superseded this one — don't clobber its status.
		if (saveVersion === expected) saveState = res.ok ? 'saved' : 'error';
		if (res.ok) {
			rebuildEmbedding();
		} else {
			console.error('Failed to save content after retries');
		}
	}

	// ── RAG: keep this page's embedding fresh on save ──
	async function rebuildEmbedding() {
		const s = await loadAISettings();
		if (!s.enabled || !s.rag || !editor) return;
		const text = htmlToMarkdown(editor.getHTML()).slice(0, 20000);
		if (!text.trim() || text === lastEmbeddedText) return;
		lastEmbeddedText = text;
		try {
			const vector = s.builtinEmbeddings
				? await embedLocal(text)
				: await embedText(s.url, s.model, text, s.apiKey);
			await invoke('upsert_embedding', { blockId: `${docId}-content`, documentId: docId, text, vector });
		} catch (e) {
			console.error('Embedding failed:', e);
		}
	}

	function handleEditorChange() {
		// Auto-title: use first heading text if title is still "Untitled".
		// state.doc.firstChild avoids a full-doc getJSON per keystroke.
		if (documentTitle === 'Untitled' || documentTitle === '') {
			const first = editor?.state.doc.firstChild;
			if (first?.type.name === 'heading') {
				const text = first.textContent;
				if (text && documentTitle !== text) {
					documentTitle = text;
					debouncedSaveTitle();
				}
			}
		}
		// Debounced auto-save
		clearTimeout(contentSaveTimer);
		contentSaveTimer = setTimeout(saveContent, 1000);
	}

	// ── Local AI (off unless enabled in Settings) ──
	$effect(() => {
		loadAISettings().then((s) => (aiEnabled = s.enabled));
	});

	interface EmbeddingRow {
		block_id: string;
		document_id: string;
		doc_title: string;
		text: string;
		vector: number[];
		updated_at: string;
	}

	/** Retrieve relevant chunks across the vault: ANN top-k from the in-DB
	 *  vector index (Rust) + FTS fallback for pages without an embedding yet.
	 *  Excludes the open page — its full content is already in the prompt.
	 *  Returns [] if embeddings are unavailable (page-only chat still works). */
	async function retrieveContext(s: AISettings, q: string) {
		const fts = await invoke<{ doc_id: string; doc_title: string; block_content: string; type: string }[]>('search_all', { query: q }).catch(() => []);
		const ftsById = new Map<string, { title: string; text: string }>();
		for (const f of fts) {
			if (f.type !== 'content' || f.doc_id === docId) continue;
			if (!ftsById.has(f.doc_id)) ftsById.set(f.doc_id, { title: f.doc_title, text: f.block_content });
		}
		let chunks: { docId: string; title: string; text: string; score: number }[] = [];
		try {
			const qVec = s.builtinEmbeddings
				? await embedLocal(q)
				: await embedText(s.url, s.model, q, s.apiKey);
			// Rust returns exact-cosine re-ranked top-k — order is the rank.
			const rows = await invoke<EmbeddingRow[]>('search_embeddings', { query: qVec, limit: 6 }).catch(() => []);
			chunks = rows
				.filter((r) => r.document_id !== docId)
				.map((r) => ({ docId: r.document_id, title: r.doc_title, text: r.text, score: 1 }));
			const embedded = new Set(chunks.map((c) => c.docId));
			for (const [id, c] of ftsById) if (!embedded.has(id)) chunks.push({ docId: id, title: c.title, text: c.text, score: 0 });
		} catch {
			// embeddings unavailable → FTS-only retrieval
			chunks = [...ftsById.entries()].map(([id, c]) => ({ docId: id, title: c.title, text: c.text, score: 0 }));
		}
		return chunks.sort((a, b) => b.score - a.score).slice(0, 6);
	}

	async function askAi() {
		const q = aiQuestion.trim();
		if (!q || aiBusy) return;
		const s = await loadAISettings();
		aiQuestion = '';
		aiBusy = true;
		aiError = '';
		aiSources = [];
		try {
			const pageMd = editor ? htmlToMarkdown(editor.getHTML()) : '';
			let systemContent = `You are a helpful assistant inside the user's personal knowledge base. Answer using the page content below when relevant, and say so when the page does not contain the answer.\n\nPage title: ${documentTitle}\n\nPage content:\n${pageMd.slice(0, 20000)}`;
			if (s.rag) {
				const ctx = await retrieveContext(s, q);
				if (ctx.length) {
					systemContent +=
						'\n\nRelevant pages from the vault (cite their page titles when you use them):\n' +
						ctx.map((c) => `[${c.title}]: ${c.text.slice(0, 1500)}`).join('\n');
					aiSources = ctx.map((c) => ({ id: c.docId, title: c.title }));
				}
			}
			const system: ChatMessage = { role: 'system', content: systemContent };
			const messages = [
				...aiMessages.filter((m) => m.role !== 'system'),
				{ role: 'user' as const, content: q },
			];
			aiMessages = [...aiMessages, { role: 'user', content: q }, { role: 'assistant', content: '' }];
			const { promise, abort } = chatStream(s.url, s.model, [system, ...messages], (d) => {
				const last = aiMessages[aiMessages.length - 1];
				aiMessages = [...aiMessages.slice(0, -1), { role: 'assistant', content: last.content + d }];
			}, s.apiKey);
			aiAbort = abort;
			await promise;
		} catch (e: any) {
			aiError = `LLM error: ${e?.message || e}`;
		} finally {
			aiBusy = false;
			aiAbort = undefined;
		}
	}

	// Mention chips navigate to their page on click.
	$effect(() => {
		const handler = (e: MouseEvent) => {
			const el = (e.target as HTMLElement).closest('[data-mention]') as HTMLElement | null;
			if (!el) return;
			e.preventDefault();
			const id = el.getAttribute('data-doc-id');
			if (id && id !== docId) {
				flushPendingSaves();
				window.location.href = `/${id}`;
			}
		};
		window.document.addEventListener('click', handler);
		return () => window.document.removeEventListener('click', handler);
	});

	async function deleteDocument() {
		if (!document) return;
		try {
			await invoke('archive_document', { id: docId });
			// Stay on the page so the undo toast can act (Notion-style);
			// the sidebar list refreshes on the next navigation.
			toast = 'Moved to trash';
			clearTimeout(toastTimer);
			toastTimer = setTimeout(() => (toast = null), 5000);
		} catch (e) {
			console.error('Failed to archive document:', e);
		}
	}

	async function undoArchive() {
		try {
			await invoke('restore_document', { id: docId });
			toast = null;
		} catch (e) {
			console.error('Failed to restore document:', e);
		}
	}

	async function toggleFavorite() {
		if (!document) return;
		try {
			document = await invoke<Document>('toggle_favorite', { id: docId });
		} catch (e) {
			console.error('Failed to toggle favorite:', e);
		}
	}

	async function exportMarkdown() {
		if (!editor) return;
		try {
			const md = htmlToMarkdown(editor.getHTML());
			exportOpen = false;
			const ok = await exportMarkdownDialog(documentTitle, md);
			if (ok) console.log('Exported');
		} catch (e) {
			console.error('Export failed:', e);
		}
	}

	async function exportHtml() {
		if (!editor) return;
		try {
			exportOpen = false;
			const ok = await exportHtmlDialog(documentTitle, editor.getHTML());
			if (ok) console.log('Exported');
		} catch (e) {
			console.error('Export failed:', e);
		}
	}

	function printPage() {
		exportOpen = false;
		try {
			// ponytail: WebKitGTK may no-op window.print — HTML export is the
			// reliable path; add a Rust PDF renderer when print is asked for.
			window.print();
		} catch (e) {
			alert('Printing is not supported in this build — use HTML export instead.');
		}
	}

	let wordCount = $derived.by(() => {
		if (!editor?.state) return 0;
		const text = editor.state.doc.textContent;
		return text.trim() ? text.trim().split(/\s+/).length : 0;
	});

	function formatDate(iso: string | undefined): string {
		if (!iso) return '—';
		const d = new Date(iso);
		if (Number.isNaN(d.getTime())) return iso;
		return d.toLocaleString();
	}

	$effect(() => {
		if (docId) {
			loading = true;
			// Reset stale content so a failed block load can't leak the
			// previous document's content into this one.
			editorContent = undefined;
			tags = [];
			tagInput = '';
			icon = '';
			cover = '';
			hasWhiteboard = false;
			loadDocument();
			loadBlocks();
			loadPageList();
			try { mode = (localStorage.getItem(`enclave-mode-${docId}`) as 'paper' | 'whiteboard') || 'paper'; } catch { mode = 'paper'; }
		}
	});

	$effect(() => {
		if (docId && editor && editor.storage.image?.docId !== docId) {
			editor.storage.image.docId = docId;
		}
	});

	$effect(() => {
		if (documentTitle && document) {
			loadBacklinks();
		}
	});
</script>

{#if loading}
	<div class="loading">Loading…</div>
{:else if document}
	<div class="document-page" class:full-width={fullWidth}>
		{#if mode === 'paper' && cover}
			<div class="doc-cover {cover}"></div>
		{/if}
		<div class="doc-topbar">
			{#if mode === 'paper'}
				<button class="icon-btn page-icon" class:has-icon={!!icon} onclick={() => (metaOpen = !metaOpen)} title="Page icon">
					{#if icon}{icon}{:else}<Icon name="smile" size={15} />{/if}
				</button>
			{/if}
			<input
				type="text"
				class="doc-title-input"
				bind:value={documentTitle}
				onblur={saveTitle}
				oninput={debouncedSaveTitle}
				placeholder="Untitled"
				aria-label="Page title"
			/>
			<div class="doc-actions">
				<span class="save-status {saveState}" role="status">
					{saveState === 'saving' ? 'Saving…' : saveState === 'error' ? 'Save failed' : ''}
				</span>
				<div class="mode-toggle" role="tablist" aria-label="Page mode">
					{#if hasWhiteboard || mode === 'whiteboard'}
						<button class="mode-btn" class:active={mode === 'paper'} onclick={() => setMode('paper')} role="tab">Paper</button>
						<button class="mode-btn" class:active={mode === 'whiteboard'} onclick={() => setMode('whiteboard')} role="tab">Whiteboard</button>
					{:else}
						<!-- A page without a whiteboard block stays paper-only; one click
						     opts this page into whiteboard mode (the block is created on
						     the first edit, so pages that never use it stay clean). -->
						<button class="mode-btn" onclick={() => setMode('whiteboard')} role="tab" title="Add a whiteboard to this page">Whiteboard</button>
					{/if}
				</div>
				<button class="icon-btn" class:active={fullWidth} onclick={toggleFullWidth} title="Toggle full width">
					<Icon name="expand" size={15} />
				</button>
				{#if mode === 'paper'}
					<button class="icon-btn split-btn" onclick={() => goto(`/split/${docId}`)} title="Split view (side-by-side pages)">
						<Icon name="layout" size={15} />
					</button>
				{/if}
				{#if mode === 'paper'}
					<button class="icon-btn" class:active={sourceOpen} onclick={toggleSource} title="Markdown source">
						<Icon name="code" size={15} />
					</button>
				{/if}
				{#if aiEnabled}
					<button class="icon-btn" class:active={aiOpen} onclick={() => (aiOpen = !aiOpen)} title="Ask AI (local)">
						<Icon name="sparkles" size={15} />
					</button>
				{/if}
				<button class="icon-btn" class:faved={document.is_favorite} onclick={toggleFavorite} title={document.is_favorite ? 'Remove from favorites' : 'Add to favorites'}>
					<Icon name="star" size={15} />
				</button>
				<div class="export-wrap">
					<button class="icon-btn" onclick={() => (infoOpen = !infoOpen)} title="Page info">
						<Icon name="info" size={15} />
					</button>
					{#if infoOpen}
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<div class="export-backdrop" onclick={() => (infoOpen = false)}></div>
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<div class="export-menu info-menu" onclick={(e: MouseEvent) => e.stopPropagation()}>
							<div class="info-row"><span>Created</span><span>{formatDate(document.created_at)}</span></div>
							<div class="info-row"><span>Modified</span><span>{formatDate(document.updated_at)}</span></div>
							<div class="info-row"><span>Words</span><span>{wordCount}</span></div>
						</div>
					{/if}
				</div>
				<div class="export-wrap">
					<button class="icon-btn" onclick={() => (exportOpen = !exportOpen)} title="Export page">
						<Icon name="download" size={15} />
					</button>
					{#if exportOpen}
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<div class="export-backdrop" onclick={() => (exportOpen = false)}></div>
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<div class="export-menu" onclick={(e: MouseEvent) => e.stopPropagation()}>
							<button class="export-item" onclick={exportMarkdown}>Markdown…</button>
							<button class="export-item" onclick={exportHtml}>HTML…</button>
							<button class="export-item" onclick={printPage}>Print (PDF)…</button>
						</div>
					{/if}
				</div>
			<button class="icon-btn danger" onclick={deleteDocument} title="Delete page">
				<Icon name="trash" size={15} />
			</button>
			<button class="icon-btn more-btn" onclick={() => (sheetOpen = true)} title="More actions" aria-label="More actions">
				<Icon name="more" size={18} />
			</button>
		</div>
	</div>

	{#if aiOpen}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="ai-backdrop" onclick={() => (aiOpen = false)}></div>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<aside class="ai-panel" role="dialog" aria-label="Ask AI" onclick={(e: MouseEvent) => e.stopPropagation()}>
			<div class="ai-head">
				<span>Ask AI</span>
				<button class="ai-close" onclick={() => (aiOpen = false)} aria-label="Close">✕</button>
			</div>
			<div class="ai-thread">
				{#each aiMessages as m (m)}
					<div class="ai-msg {m.role}">{m.content || (aiBusy ? '…' : '')}</div>
					{#if m.role === 'assistant' && m === aiMessages[aiMessages.length - 1] && aiSources.length}
						<div class="ai-sources">
							<span class="ai-sources-label">Sources:</span>
							{#each aiSources as src (src.id)}
								<a class="ai-source" href="/{src.id}" onclick={() => (aiOpen = false)}>{src.title}</a>
							{/each}
						</div>
					{/if}
				{/each}
				{#if aiError}
					<div class="ai-err" role="alert">{aiError}</div>
				{/if}
				{#if aiMessages.length === 0}
					<div class="ai-empty">Ask a question about this page — or, with RAG on, across your whole vault. Runs on your device.</div>
				{/if}
			</div>
			<div class="ai-compose">
				<input
					class="ai-input"
					bind:value={aiQuestion}
					placeholder="Ask about this page…"
					aria-label="Ask about this page"
					disabled={aiBusy}
					onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') { e.preventDefault(); askAi(); } }}
				/>
				{#if aiBusy}
					<button class="ai-stop" onclick={() => aiAbort?.()}>Stop</button>
				{:else}
					<button class="ai-send" onclick={askAi} disabled={!aiQuestion.trim()}>Ask</button>
				{/if}
			</div>
		</aside>
	{/if}

		{#if mode === 'paper' && metaOpen}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="meta-backdrop" onclick={() => (metaOpen = false)}></div>
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div class="meta-popover" onclick={(e: MouseEvent) => e.stopPropagation()}>
				<div class="meta-section-title">Page icon</div>
				<EmojiPicker value={icon} onPick={setIcon} />
				<div class="meta-section-title">Cover</div>
				<div class="cover-row">
					{#each COVERS as c}
						<button
							class="cover-swatch {c}"
							class:selected={cover === c}
							onclick={() => setCover(c)}
							aria-label={`Cover ${c}`}
						></button>
					{/each}
					{#if cover}
						<button class="cover-remove" onclick={() => setCover('')} title="Remove cover">✕</button>
					{/if}
				</div>
			</div>
		{/if}

	<div class="doc-tags">
		{#each tags as t (t)}
			<span class="tag-chip">
				<span class="tag-hash">#</span>{t}
				<button class="tag-x" aria-label={`Remove tag ${t}`} onclick={() => removeTag(t)}>✕</button>
			</span>
		{/each}
		<input
			class="tag-input"
			placeholder={tags.length ? 'Add tag…' : 'Add tags…'}
			bind:value={tagInput}
			onkeydown={(e: KeyboardEvent) => {
				if (e.key === 'Enter' || e.key === ',') {
					e.preventDefault();
					addTag();
				} else if (e.key === 'Backspace' && tagInput === '' && tags.length > 0) {
					removeTag(tags[tags.length - 1]);
				}
			}}
			onblur={() => { if (tagInput.trim()) addTag(); }}
			aria-label="Add tag"
		/>
	</div>

	{#if mode === 'paper'}
		<div class="doc-body">
			{#if sourceOpen}
				<textarea
					class="source-view"
					bind:value={sourceText}
					spellcheck="false"
					aria-label="Markdown source"
					oninput={() => (sourceDirty = true)}
					onkeydown={(e: KeyboardEvent) => {
						if ((e.ctrlKey || e.metaKey) && e.key === 's') {
							e.preventDefault();
							applySource();
						}
					}}
				></textarea>
			{:else}
				<div class="doc-editor">
					<TipTapEditor
						bind:editor
						content={editorContent}
						placeholder="Type / for commands…"
						autofocus
						onChange={handleEditorChange}
					/>
					<SlashMenu {editor} />
					<BubbleMenu {editor} />
					<PageLinkMenu {editor} allPages={pageList} />
					<MentionMenu {editor} allPages={pageList} />
					<DragHandleMenu {editor} />
					<EditorContextMenu {editor} allPages={pageList} />
					<TableMenu {editor} />
				</div>

				{#if backlinks.length > 0}
					<aside class="backlinks-panel">
						<div class="backlinks-header">Backlinks ({backlinks.length})</div>
						{#each backlinks as bl}
							<a href="/{bl.doc_id}" class="backlink-item">
								<span class="backlink-doc">{bl.doc_title}</span>
								<span class="backlink-content">{bl.block_content.slice(0, 100)}</span>
							</a>
						{/each}
					</aside>
				{/if}

				<TocPanel {editor} />
			{/if}
			</div>
		{:else}
			<Whiteboard docId={docId!} />
		{/if}

		<!-- Comments moved below the editor: they annotate the page, they
		     shouldn't sit between the title and the content. -->
		<div class="doc-comments">
			{#each comments as c (c.id)}
				<div class="comment-item">
					<div class="comment-text">{c.text}</div>
					<div class="comment-meta">
						<span>{new Date(c.at).toLocaleString()}</span>
						<button class="comment-del" aria-label="Delete comment" onclick={() => removeComment(c.id)}>✕</button>
					</div>
				</div>
			{/each}
			{#if comments.length === 0}
				<div class="comments-empty">No comments yet.</div>
			{/if}
			<div class="comment-add">
				<input
					class="comment-input"
					bind:value={commentInput}
					placeholder="Write a comment…"
					aria-label="Write a comment"
					onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') { e.preventDefault(); addComment(); } }}
				/>
				<button class="comment-submit" onclick={addComment} disabled={!commentInput.trim()}>Add</button>
			</div>
		</div>

		{#if toast}
			<div class="toast" role="status">
				<span>{toast}</span>
				<button class="toast-undo" onclick={undoArchive}>Undo</button>
			</div>
		{/if}

		<ActionSheet bind:open={sheetOpen} title="Page actions" items={sheetItems} />
	</div>
{:else}
	<div class="empty-state">
		<p>Document not found.</p>
		<a href="/">Go home</a>
	</div>
{/if}

<style>
	.loading {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--color-text-muted);
	}

	.doc-tags {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 6px;
		padding: 0 40px 4px;
	}

	.tag-chip {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		border-radius: 999px;
		background: rgba(124, 111, 240, 0.12);
		color: #9d8cff;
		font-size: 12px;
		padding: 2px 10px;
	}

	.tag-hash {
		opacity: 0.7;
	}

	.tag-x {
		border: none;
		background: none;
		color: inherit;
		opacity: 0.6;
		cursor: pointer;
		font-size: 10px;
		padding: 0;
	}

	.tag-x:hover {
		opacity: 1;
	}

	.tag-input {
		border: none;
		background: none;
		color: var(--color-text-muted);
		font-size: 12px;
		outline: none;
		width: 100px;
	}

	.document-page {
		max-width: var(--page-width, 1100px);
		margin: 0 auto;
		padding: 0 48px;
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.document-page.full-width {
		max-width: 1600px;
	}

	/* ── Cover ── */
	.doc-cover {
		height: 140px;
		border-radius: var(--radius-lg);
		margin-top: 16px;
		flex-shrink: 0;
	}

	.grad-0 { background: linear-gradient(135deg, #7c6ff0, #4fc3f7); }
	.grad-1 { background: linear-gradient(135deg, #f0a45c, #f0608c); }
	.grad-2 { background: linear-gradient(135deg, #57d3a3, #2f9e77); }
	.grad-3 { background: linear-gradient(135deg, #8b5cf6, #ec4899); }
	.grad-4 { background: linear-gradient(135deg, #f59e0b, #ef4444); }
	.grad-5 { background: linear-gradient(135deg, #3b82f6, #06b6d4); }

	/* ── Page icon & meta popover ── */
	.page-icon {
		font-size: 20px;
		flex-shrink: 0;
		width: 36px;
		height: 36px;
		border-radius: 8px;
	}

	.page-icon.has-icon { background: var(--color-surface-hover); }

	.meta-popover {
		position: fixed;
		z-index: 320;
		top: 76px;
		left: 50%;
		transform: translateX(-50%);
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 12px;
		box-shadow: var(--shadow-lg);
		padding: 12px 14px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.meta-backdrop {
		position: fixed;
		inset: 0;
		z-index: 310;
		background: var(--color-overlay);
	}

	.meta-section-title {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-faint);
	}

	.cover-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.cover-swatch {
		width: 36px;
		height: 36px;
		border-radius: 8px;
		border: 2px solid transparent;
		cursor: pointer;
		padding: 0;
	}

	.cover-swatch.selected {
		border-color: var(--color-text);
	}

	.cover-remove {
		border: none;
		background: none;
		color: var(--color-text-muted);
		cursor: pointer;
		font-size: 12px;
	}

	/* ── Export menu ── */
	.export-wrap {
		position: relative;
		display: flex;
	}

	.export-backdrop {
		position: fixed;
		inset: 0;
		z-index: 290;
	}

	.export-menu {
		position: absolute;
		z-index: 291;
		top: calc(100% + 4px);
		right: 0;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
		padding: 5px;
		min-width: 150px;
	}

	.export-item {
		display: block;
		width: 100%;
		border: none;
		background: none;
		color: var(--color-text);
		font-size: 13px;
		font-family: inherit;
		text-align: left;
		padding: 6px 10px;
		border-radius: 6px;
		cursor: pointer;
	}

	.export-item:hover {
		background: var(--color-surface-hover);
	}

	.info-menu {
		width: 220px;
		padding: 8px 10px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.info-row {
		display: flex;
		justify-content: space-between;
		gap: 12px;
		font-size: 12px;
	}

	.info-row span:first-child {
		color: var(--color-text-muted);
	}

	.info-row span:last-child {
		color: var(--color-text);
		text-align: right;
	}

	/* ── Toast ── */
	.toast {
		position: fixed;
		bottom: 24px;
		left: 50%;
		transform: translateX(-50%);
		z-index: 400;
		display: flex;
		align-items: center;
		gap: 12px;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		box-shadow: var(--shadow-lg);
		padding: 10px 16px;
		font-size: 13px;
		color: var(--color-text);
	}

	.toast-undo {
		border: none;
		background: none;
		color: var(--color-accent);
		cursor: pointer;
		font-size: 13px;
		font-weight: 600;
		font-family: inherit;
		padding: 0;
	}

	.toast-undo:hover {
		text-decoration: underline;
	}

	.icon-btn.active {
		color: var(--color-accent);
		background: var(--color-accent-subtle);
	}

	.doc-topbar {
		padding: 20px 0 10px;
		display: flex;
		align-items: center;
		gap: 16px;
		flex-shrink: 0;
	}

	.doc-title-input {
		flex: 1;
		min-width: 0;
		font-size: 30px;
		font-weight: 700;
		letter-spacing: -0.02em;
		color: var(--color-text);
		background: none;
		border: none;
		outline: none;
		font-family: inherit;
		padding: 4px 0;
	}
	.doc-title-input::placeholder { color: var(--color-text-faint); }

	.doc-actions {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-shrink: 0;
	}
	.save-status {
		font-size: 11px;
		color: var(--color-text-faint);
		flex-shrink: 0;
		align-self: center;
		white-space: nowrap;
	}
	.save-status.error { color: var(--color-danger); font-weight: 500; }

	.mode-toggle {
		display: flex;
		background: var(--color-surface-hover);
		border-radius: var(--radius-md);
		padding: 2px;
		margin-right: 8px;
	}
	.mode-btn {
		border: none;
		background: none;
		color: var(--color-text-muted);
		font-size: 12px;
		font-family: inherit;
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: background 0.1s, color 0.1s;
	}
	.mode-btn.active {
		background: var(--color-surface-active);
		color: var(--color-text);
		font-weight: 500;
	}

	.icon-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 30px;
		height: 30px;
		border: none;
		border-radius: var(--radius-md);
		background: none;
		color: var(--color-text-muted);
		cursor: pointer;
		padding: 0;
		transition: background 0.1s, color 0.1s;
	}
	.icon-btn:hover { background: var(--color-surface-hover); color: var(--color-text); }
	.icon-btn.faved { color: var(--color-warning); }
	.icon-btn.danger:hover { color: var(--color-danger); background: rgba(229, 83, 75, 0.1); }

	.doc-comments {
		max-width: 720px;
		margin: 0 auto 24px;
		padding: 0 40px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.comments-head {
		font-size: 12px; font-weight: 600; text-transform: uppercase;
		letter-spacing: 0.05em; color: var(--color-text-faint);
		margin-top: 12px;
	}
	.comment-item {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		padding: 10px 12px;
	}
	.comment-text { font-size: 14px; line-height: 1.5; white-space: pre-wrap; word-break: break-word; }
	.comment-meta {
		display: flex; justify-content: space-between; align-items: center;
		margin-top: 6px; font-size: 11px; color: var(--color-text-faint);
	}
	.comment-del {
		background: none; border: none; color: var(--color-text-faint);
		cursor: pointer; font-size: 12px; padding: 2px 4px; border-radius: 4px;
	}
	.comment-del:hover { background: rgba(229, 83, 75, 0.12); color: var(--color-danger); }
	.comments-empty { font-size: 13px; color: var(--color-text-faint); }
	.comment-add { display: flex; gap: 8px; }
	.comment-input {
		flex: 1;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		font-size: 14px;
		font-family: inherit;
		padding: 8px 12px;
		outline: none;
	}
	.comment-input:focus { border-color: var(--color-accent); }
	.comment-submit {
		background: var(--color-accent); color: #fff;
		border: none; border-radius: var(--radius-md);
		padding: 8px 16px; font-size: 13px; font-weight: 500;
		cursor: pointer; font-family: inherit;
	}
	.comment-submit:disabled { opacity: 0.5; cursor: default; }

	/* ── Ask AI panel ── */
	.ai-backdrop {
		position: fixed; inset: 0; z-index: 260;
		background: var(--color-overlay);
	}
	.ai-panel {
		position: fixed; top: 12px; right: 12px; bottom: 12px; z-index: 261;
		width: 360px; max-width: 90vw;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-lg);
		display: flex; flex-direction: column;
		overflow: hidden;
	}
	.ai-head {
		display: flex; align-items: center; justify-content: space-between;
		padding: 12px 16px; border-bottom: 1px solid var(--color-border);
		font-size: 14px; font-weight: 600;
	}
	.ai-close {
		background: none; border: none; color: var(--color-text-muted);
		cursor: pointer; font-size: 14px; padding: 2px 4px;
	}
	.ai-thread {
		flex: 1; overflow-y: auto; padding: 12px 16px;
		display: flex; flex-direction: column; gap: 8px;
	}
	.ai-msg {
		font-size: 13px; line-height: 1.55; white-space: pre-wrap; word-break: break-word;
		padding: 8px 12px; border-radius: var(--radius-md);
	}
	.ai-msg.user { align-self: flex-end; background: var(--color-accent-subtle); }
	.ai-msg.assistant { align-self: flex-start; background: var(--color-surface-hover); }
	.ai-err { font-size: 12px; color: var(--color-danger); }
	.ai-sources { display: flex; align-items: center; flex-wrap: wrap; gap: 4px 8px; font-size: 12px; color: var(--color-text-faint); }
	.ai-source { color: var(--color-accent); text-decoration: none; }
	.ai-source:hover { text-decoration: underline; }
	.ai-empty { font-size: 12px; color: var(--color-text-faint); text-align: center; padding: 20px 0; }
	.ai-compose {
		display: flex; gap: 8px; padding: 12px 16px;
		border-top: 1px solid var(--color-border);
	}
	.ai-input {
		flex: 1;
		background: var(--color-surface-hover);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		font-size: 13px; font-family: inherit;
		padding: 8px 12px; outline: none;
	}
	.ai-input:focus { border-color: var(--color-accent); }
	.ai-input:disabled { opacity: 0.6; }
	.ai-send, .ai-stop {
		border: none; border-radius: var(--radius-md);
		padding: 8px 14px; font-size: 13px; font-weight: 500; cursor: pointer; font-family: inherit;
	}
	.ai-send { background: var(--color-accent); color: #fff; }
	.ai-send:disabled { opacity: 0.5; cursor: default; }
	.ai-stop { background: var(--color-surface-active); color: var(--color-text); }

	.doc-body {		display: flex;
		gap: 32px;
		flex: 1;
		min-height: 0;
	}

	.doc-editor {
		flex: 1;
		min-width: 0;
		overflow-y: auto;
		padding-bottom: 48px;
	}

	/* ── Markdown source view ── */
	.source-view {
		flex: 1;
		min-width: 0;
		min-height: 100%;
		width: 100%;
		box-sizing: border-box;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		color: var(--color-text);
		font-family: var(--font-mono);
		font-size: 13px;
		line-height: 1.65;
		padding: 16px 18px;
		resize: none;
		outline: none;
	}
	.source-view:focus {
		border-color: var(--color-accent);
	}

	.backlinks-panel {
		width: 220px;
		flex-shrink: 0;
		border-left: 1px solid var(--color-border);
		padding-left: 20px;
		padding-top: 12px;
		overflow-y: auto;
	}
	.backlinks-header {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-text-faint);
		margin-bottom: 12px;
	}
	.backlink-item {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 8px;
		border-radius: var(--radius-md);
		color: var(--color-text-muted);
		text-decoration: none;
		font-size: 13px;
		transition: background 0.1s;
		margin-bottom: 4px;
	}
	.backlink-item:hover { background: var(--color-surface-hover); color: var(--color-text); }
	.backlink-doc { font-size: 13px; font-weight: 500; color: var(--color-text); }
	.backlink-content {
		font-size: 12px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		gap: 12px;
		color: var(--color-text-muted);
	}
	.empty-state a { color: var(--color-accent); text-decoration: none; }

	.more-btn {
		display: none;
	}

	/* ── Phone layout ── */
	@media (max-width: 768px) {
		/* Split view is a desktop workspace; the split route still works
		   stacked on phones, reached via URL or Ctrl+Click. */
		.split-btn { display: none; }
		.document-page { padding: 0 14px; }

		/* HCI: the topbar icon cluster is too cramped for thumbs — collapse
		   everything but the essentials into the ⋯ action sheet. */
		.doc-actions .save-status,
		.doc-actions .icon-btn:not(.more-btn) {
			display: none;
		}
		.doc-actions .more-btn {
			display: flex;
			width: 44px;
			height: 44px;
		}
		.doc-actions {
			margin-left: auto;
			gap: 2px;
		}
		.mode-btn {
			padding: 8px 12px;
			font-size: 13px;
		}
		.doc-title-input {
			font-size: 24px;
		}

		/* Page-info popover becomes a glass bottom sheet on phones. */
		.export-menu {
			position: fixed;
			left: 12px;
			right: 12px;
			bottom: calc(16px + env(safe-area-inset-bottom));
			top: auto;
			background: color-mix(in srgb, var(--color-surface) 62%, transparent);
			backdrop-filter: blur(28px) saturate(170%);
			-webkit-backdrop-filter: blur(28px) saturate(170%);
			border: 1px solid color-mix(in srgb, var(--color-border) 55%, transparent);
			border-radius: 18px;
			box-shadow: 0 8px 36px rgba(0, 0, 0, 0.4);
		}
		.doc-topbar { flex-wrap: wrap; gap: 6px 10px; padding-top: 12px; }
		.doc-title-input { font-size: 24px; }
		.doc-actions { margin-left: auto; flex-wrap: wrap; justify-content: flex-end; }
		.mode-toggle { margin-right: 0; }
		.doc-cover { height: 96px; margin-top: 10px; }

		.doc-tags { padding: 0 2px 4px; }
		.doc-comments { padding: 0 2px; }

		/* The 220px backlinks rail has no room on a phone. */
		.backlinks-panel { display: none; }

		/* Ask-AI panel becomes a full-screen sheet. */
		.ai-panel {
			inset: 0;
			width: 100%;
			max-width: 100%;
			border-radius: 0;
			border: none;
			padding-top: env(safe-area-inset-top);
		}
		.ai-compose { padding-bottom: calc(12px + env(safe-area-inset-bottom)); }

		/* Page-icon popover: full-width instead of centered-overflowing. */
		.meta-popover {
			left: 12px;
			right: 12px;
			transform: none;
			top: 60px;
		}
	}
</style>
