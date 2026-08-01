// Minimal Ollama HTTP client — no dependency, talks to the local Ollama
// instance (http://localhost:11434 by default) over the existing CSP
// allowance. Pure parsing lives in parseChunks so it's unit-testable.

export interface ChatMessage {
	role: 'system' | 'user' | 'assistant';
	content: string;
}

export interface AISettings {
	enabled: boolean;
	url: string;
	model: string;
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
	return { enabled: false, url: DEFAULT_URL, model: 'llama3.2' };
}

export function saveAISettings(s: AISettings) {
	try { localStorage.setItem('enclave-ai', JSON.stringify(s)); } catch { /* ignore */ }
}

export async function listModels(url: string): Promise<string[]> {
	const res = await fetch(`${url}/api/tags`);
	if (!res.ok) throw new Error(`Ollama error ${res.status}`);
	const data = await res.json();
	return (data.models ?? []).map((m: { name: string }) => m.name);
}

/** Consume a buffer of Ollama streaming JSON lines; return complete deltas
 *  and the unparsed tail to keep for the next chunk. */
export function parseChunks(buf: string): { deltas: string[]; rest: string } {
	const lines = buf.split('\n');
	const rest = lines.pop() ?? '';
	const deltas: string[] = [];
	for (const line of lines) {
		const t = line.trim();
		if (!t) continue;
		try {
			const j = JSON.parse(t);
			if (j.message?.content) deltas.push(j.message.content);
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
		const res = await fetch(`${url}/api/chat`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ model, messages, stream: true }),
			signal: controller.signal,
		});
		if (!res.ok) throw new Error(`Ollama error ${res.status}`);
		const reader = res.body!.getReader();
		const dec = new TextDecoder();
		let buf = '';
		for (;;) {
			const { done, value } = await reader.read();
			if (done) break;
			buf += dec.decode(value, { stream: true });
			const { deltas, rest } = parseChunks(buf);
			buf = rest;
			for (const d of deltas) onDelta(d);
		}
	})();
	return { promise, abort: () => controller.abort() };
}
