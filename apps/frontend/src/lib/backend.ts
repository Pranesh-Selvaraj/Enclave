// Backend bridge — the frontend only ever runs inside the Tauri desktop
// shell; there is no web build. Re-exporting invoke/listen here keeps the
// rest of the app decoupled from @tauri-apps/api and gives one place to
// audit IPC surface.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export { invoke, listen };
