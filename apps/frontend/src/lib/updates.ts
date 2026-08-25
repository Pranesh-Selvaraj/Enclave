// Offline-first update policy. Enclave never phones home on its own: the
// user must opt in ("allow update checks") and then approve every update
// individually after reading its changelog. The pref lives in localStorage
// like the theme settings — nothing here touches the network.
import { invoke } from './backend.js';

export type UpdateInfo = {
	current_version: string;
	latest_version: string;
	update_available: boolean;
	notes: string;
	asset_name: string | null;
	asset_url: string | null;
	asset_size: number | null;
};

const KEY = 'enclave-update-prefs';

export function loadUpdatePrefs(): boolean {
	try {
		return localStorage.getItem(KEY) === '1';
	} catch {
		return false;
	}
}

export function saveUpdatePrefs(enabled: boolean) {
	try {
		localStorage.setItem(KEY, enabled ? '1' : '0');
	} catch { /* private browsing */ }
}

export function checkForUpdate(): Promise<UpdateInfo> {
	return invoke('check_for_update');
}

export function downloadUpdate(url: string, filename: string): Promise<string> {
	return invoke('download_update', { url, filename });
}

export function installUpdate(path: string): Promise<void> {
	return invoke('install_update', { path });
}
