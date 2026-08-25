// Save with backoff retry, guarded by a version token. The guard matters:
// saves are debounced, so a slow retry of a stale payload must never
// overwrite a newer one (blocks upsert by id — last write wins).
export interface SaveResult {
	ok: boolean;
	/** The version token observed at the end of the attempt. */
	version: number;
}

export async function saveWithRetry(
	save: () => Promise<void>,
	version: () => number,
	expectedVersion: number,
	attempts = 3,
	baseDelayMs = 1000,
): Promise<SaveResult> {
	for (let attempt = 0; attempt < attempts; attempt++) {
		try {
			await save();
			return { ok: true, version: version() };
		} catch (e) {
			// Give up if we ran out of attempts or a newer save superseded us.
			if (attempt === attempts - 1 || version() !== expectedVersion) {
				return { ok: false, version: version() };
			}
			await new Promise((r) => setTimeout(r, baseDelayMs * (attempt + 1)));
		}
	}
	// Unreachable — the loop always returns.
	return { ok: false, version: version() };
}
