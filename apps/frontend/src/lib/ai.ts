// OpenAI-compatible AI client — no dependency. Talks to any /v1 endpoint:
// local servers (Ollama, llama.cpp/llama-server, LM Studio, vLLM) or remote
// frontier APIs (OpenAI, Anthropic-compatible gateways, …) with an API key.
// Also exposes the built-in offline embedding path (Rust/ONNX) for RAG that
// needs no external service at all.
// Pure parsing lives in parseSSE so it's unit-testable.

// Import invoke directly (same symbol backend.ts re-exports) so this module
// stays unit-testable from Node without SvelteKit's `$lib` alias resolution.
import { invoke } from '@tauri-apps/api/core';

export interface ChatMessage {
	role: 'system' | 'user' | 'assistant';
	content: string;
}

export interface AISettings {
	enabled: boolean;
	/** OpenAI-compatible endpoint base URL (chat + remote embeddings). */
	url: string;
	model: string;
	/** Optional Bearer key for frontier APIs; stored encrypted in the vault. */
	apiKey: string;
	/** Vault-wide RAG retrieval (embeddings + FTS fallback). */
	rag: boolean;
	/** Embed with the built-in offline ONNX model instead of the endpoint. */
	builtinEmbeddings: boolean;
}

const DEFAULT_URL = 'http://localhost:11434';
/** Legacy localStorage key (pre-vault settings) — read once, then migrated. */
const LEGACY_KEY = 'enclave-ai';
/** Vault settings key — encrypted at rest by SQLCipher. */
const VAULT_KEY = 'ai';

export function defaultAISettings(): AISettings {
	return { enabled: false, url: DEFAULT_URL, model: 'llama3.2', apiKey: '', rag: true, builtinEmbeddings: false };
}

/** Load AI settings from the encrypted vault; migrates the old localStorage
 *  copy on first run (pre-vault-settings vaults keep their config). */
export async function loadAISettings(): Promise<AISettings> {
	try {
		const stored = await invoke<string | null>('get_setting', { key: VAULT_KEY });
		if (stored) return { ...defaultAISettings(), ...JSON.parse(stored) };
	} catch { /* vault locked or corrupt — fall through to defaults */ }
	// Legacy migration
	try {
		const raw = localStorage.getItem(LEGACY_KEY);
		if (raw) {
			const s = { ...defaultAISettings(), ...JSON.parse(raw) };
			void saveAISettings(s);
			return s;
		}
	} catch { /* ignore */ }
	return defaultAISettings();
}

export async function saveAISettings(s: AISettings) {
	try {
		await invoke('set_setting', { key: VAULT_KEY, value: JSON.stringify(s) });
		localStorage.removeItem(LEGACY_KEY);
	} catch { /* vault locked — settings simply won't persist */ }
}

/** Shared headers for endpoint calls: JSON + optional Bearer key. */
function headers(apiKey: string): Record<string, string> {
	const h: Record<string, string> = { 'Content-Type': 'application/json' };
	if (apiKey) h.Authorization = `Bearer ${apiKey}`;
	return h;
}

export async function listModels(url: string, apiKey = ''): Promise<string[]> {
	const res = await fetch(`${url}/v1/models`, { headers: headers(apiKey) });
	if (!res.ok) throw new Error(`LLM error ${res.status}`);
	const data = await res.json();
	return (data.data ?? []).map((m: { id: string }) => m.id);
}

/** Consume a buffer of SSE `data:` lines; return complete deltas and the
 *  unparsed tail to keep for the next chunk. */
export function parseSSE(buf: string): { deltas: string[]; rest: string } {
	const lines = buf.split('\n');
	const rest = lines.pop() ?? '';
	const deltas: string[] = [];
	for (const line of lines) {
		const t = line.trim();
		if (!t.startsWith('data:')) continue;
		const payload = t.slice(5).trim();
		if (payload === '[DONE]') continue;
		try {
			const j = JSON.parse(payload);
			const d = j.choices?.[0]?.delta?.content;
			if (d) deltas.push(d);
		} catch {
			// truncated line — it lives in `rest` (or is noise)
		}
	}
	return { deltas, rest };
}

/** Stream a chat completion, calling onDelta per token. Returns an abort
 *  function so the UI can stop generation. */
export function chatStream(
	url: string,
	model: string,
	messages: ChatMessage[],
	onDelta: (delta: string) => void,
	apiKey = '',
): { promise: Promise<void>; abort: () => void } {
	const controller = new AbortController();
	const promise = (async () => {
		const res = await fetch(`${url}/v1/chat/completions`, {
			method: 'POST',
			headers: { ...headers(apiKey), 'Accept': 'text/event-stream' },
			body: JSON.stringify({ model, messages, stream: true }),
			signal: controller.signal,
		});
		if (!res.ok) throw new Error(`LLM error ${res.status}`);
		const reader = res.body!.getReader();
		const dec = new TextDecoder();
		let buf = '';
		for (;;) {
			const { done, value } = await reader.read();
			if (done) break;
			buf += dec.decode(value, { stream: true });
			const { deltas, rest } = parseSSE(buf);
			buf = rest;
			for (const d of deltas) onDelta(d);
		}
	})();
	return { promise, abort: () => controller.abort() };
}

/** Embed a text via the configured endpoint's /v1/embeddings. */
export async function embedText(url: string, model: string, text: string, apiKey = ''): Promise<number[]> {
	const res = await fetch(`${url}/v1/embeddings`, {
		method: 'POST',
		headers: headers(apiKey),
		body: JSON.stringify({ model, input: text }),
	});
	if (!res.ok) throw new Error(`LLM error ${res.status}`);
	const data = await res.json();
	const emb = data?.data?.[0]?.embedding;
	if (!Array.isArray(emb)) throw new Error('No embedding in response');
	return emb;
}

/** Embed a text with the built-in offline ONNX model (no endpoint needed). */
export async function embedLocal(text: string): Promise<number[]> {
	return invoke('embed_text', { text });
}

