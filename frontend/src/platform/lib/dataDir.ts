// Resolves OA's data dir — portable mode (`<exe_dir>/settings/`) or AppData,
// per the Rust-side `data_dir::resolve_data_dir`. Returns the absolute path
// to use as the prefix for asset-protocol URLs (cover art, screenshots).
//
// Frontend modules MUST use this instead of `@tauri-apps/api/path::appDataDir()`
// — `appDataDir()` always returns the OS AppData path (Tauri's resolver doesn't
// know about portable mode), which would be wrong in portable installs.
//
// Cached: the dir doesn't change during a session, so we invoke once and
// every caller shares the same Promise.

import { getOaDataDir } from "@oa/platform/api/shellApi";

let cached: Promise<string> | undefined;

export function getDataDir(): Promise<string> {
  if (cached === undefined) {
    cached = getOaDataDir().catch((e) => {
      // Reset cache on failure so the next call retries — useful during
      // dev when the backend restarts mid-session.
      cached = undefined;
      throw e;
    });
  }
  return cached;
}
