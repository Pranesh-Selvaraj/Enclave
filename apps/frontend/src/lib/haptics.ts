// Haptic feedback for phones. Honours the user's vibration setting and
// silently no-ops on desktop (navigator.vibrate exists only in mobile
// WebViews).
import { theme } from '@enclave/ui';

export function haptic(pattern: number | number[] = 10) {
	if (!theme.haptics) return;
	try {
		navigator.vibrate?.(pattern);
	} catch { /* unsupported */ }
}
