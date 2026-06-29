import { createSubscriber } from 'svelte/reactivity';
import type { Editor } from '@tiptap/core';

/**
 * Wraps a TipTap Editor instance with Svelte 5 reactivity.
 * Uses a Proxy to track property access and re-render on transactions.
 */
export function makeReactive(editor: Editor): Editor {
	const subscribe = createSubscriber((update) => {
		editor.on('transaction', update);
		return () => {
			editor.off('transaction', update);
		};
	});

	return new Proxy(editor, {
		get(target, property, receiver) {
			subscribe();
			return Reflect.get(target, property, receiver);
		}
	});
}
