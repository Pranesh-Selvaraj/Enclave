// Minimal OpenAI-compatible LLM client — no dependency, talks to any local
// endpoint that exposes /v1/chat/completions (SSE) and /v1/embeddings:
// Ollama (http://localhost:11434/v1 by default), llama.cpp (llama-server),
// LM Studio, vllm, … over the existing CSP allowance (localhost).
// Pure parsing lives in parseSSE so it's unit-testable.

export interface ChatMessage {
	role: 'system' | 'user' | 'assistant';
	content: string;
}

export interface AISettings {
	enabled: boolean;
	url: string;
	model: string;
	/** Vault-wide RAG retrieval (embeddings + FTS fallback). */
	rag: boolean;
}

const DEFAULT_URL = 'http://localhost:11434';

export function loadAISettings(): AISettings {
	try {
		const raw = localStorage.getItem('enclave-ai');
		if (raw) return { ...defaultAISettings(), ...JSON.parse(raw) };
	} catch { /* ignore */ }
	return defaultAISettings();
}

export function defaultAISettings(): AISettings {
	return { enabled: false, url: DEFAULT_URL, model: 'llama3.2', rag: true };
}

export function saveAISettings(s: AISettings) {
	try { localStorage.setItem('enclave-ai', JSON.stringify(s)); } catch { /* ignore */ }
}

export async function listModels(url: string): Promise<string[]> {
	const res = await fetch(`${url}/v1/models`);
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
): { promise: Promise<void>; abort: () => void } {
	const controller = new AbortController();
	const promise = (async () => {
		const res = await fetch(`${url}/v1/chat/completions`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
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

/** Embed a text as a vector via /v1/embeddings. */
export async function embedText(url: string, model: string, text: string): Promise<number[]> {
	const res = await fetch(`${url}/v1/embeddings`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ model, input: text }),
	});
	if (!res.ok) throw new Error(`LLM error ${res.status}`);
	const data = await res.json();
	const emb = data?.data?.[0]?.embedding;
	if (!Array.isArray(emb)) throw new Error('No embedding in response');
	return emb;
}

/** Cosine similarity between two equal-length vectors; 0 for empty/mismatched. */
export function cosineSimilarity(a: number[], b: number[]): number {
	if (a.length === 0 || a.length !== b.length) return 0;
	let dot = 0, na = 0, nb = 0;
	for (let i = 0; i < a.length; i++) {
		dot += a[i] * b[i];
		na += a[i] * a[i];
		nb += b[i] * b[i];
	}
	const denom = Math.sqrt(na) * Math.sqrt(nb);
	return denom === 0 ? 0 : dot / denom;
}
