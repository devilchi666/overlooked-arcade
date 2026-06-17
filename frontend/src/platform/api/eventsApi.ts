// Typed Tauri bridge — events domain (the `oa://…` broadcast channel).
//
// Theming Phase 4.5 (the event corral, sibling to the invoke ban). This is the
// ONE module allowed to import `listen` / `emit` / `once` from
// `@tauri-apps/api/event`; everywhere else subscribes/publishes through the
// typed helpers here, and references an event NAME via the `OA_EVENTS` registry
// rather than a raw `"oa://…"` string. A `no-restricted-imports` rule
// (eslint.config.mjs) bans the raw event API outside `src/platform/api/**`, so a
// theme can't hard-wire to a backend event name any more than it can to a
// backend command name.
//
// The registry is the single source of truth for every event-name string —
// rename an event in one place and every subscriber/publisher follows. Payloads
// stay generic on `<T>` (each call site declares the shape it reads, same
// convention as the invoke wrappers' generic getters).

import { listen, emit, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";
import { onCleanup } from "solid-js";

/// Every `oa://…` broadcast channel the Rust backend (or the frontend itself)
/// emits, keyed by a camelCase name. The ONLY place these strings live.
export const OA_EVENTS = {
  audioPlaybackFailed: "oa://audio-playback-failed",
  coreDownloadProgress: "oa://core-download-progress",
  externalSessionEnded: "oa://external-session-ended",
  gameFocusChanged: "oa://game-focus-changed",
  jobEvent: "oa://job-event",
  libraryMetadataSync: "oa://library-metadata-sync",
  libraryMetadataSyncComplete: "oa://library-metadata-sync-complete",
  libraryScanComplete: "oa://library-scan-complete",
  libraryScanProgress: "oa://library-scan-progress",
  librarySync: "oa://library-sync",
  librarySyncComplete: "oa://library-sync-complete",
  libraryWatchFound: "oa://library-watch-found",
  libraryWatchRemoved: "oa://library-watch-removed",
  mediaUpdated: "oa://media-updated",
  milestoneTriggered: "oa://milestone-triggered",
  platformMediaUpdated: "oa://platform-media-updated",
  requestQuickSettings: "oa://request-quick-settings",
  romHashResolveComplete: "oa://rom-hash-resolve-complete",
  romHashResolveProgress: "oa://rom-hash-resolve-progress",
  romHashesSynced: "oa://rom-hashes-synced",
  romUnloaded: "oa://rom-unloaded",
  shaderPresetsChanged: "oa://shader-presets-changed",
  toast: "oa://toast",
  windowShown: "oa://window-shown",
} as const;

/// One of the registered event-name strings.
export type OaEventName = (typeof OA_EVENTS)[keyof typeof OA_EVENTS];

/// Subscribe to an event for the lifetime of the current Solid tracking scope.
/// Bakes in `onCleanup` so callers don't track the UnlistenFn by hand. Must be
/// called SYNCHRONOUSLY inside a Solid scope (onMount body, component setup,
/// createEffect) — calling after an `await` loses the tracking owner.
///
/// Race safety: if the scope tears down before `listen()` resolves, we flip
/// `cancelled` and unlisten as soon as the Promise completes — no leak.
export function listenScoped<T>(channel: OaEventName, handler: EventCallback<T>): void {
  let cancelled = false;
  let unlisten: (() => void) | undefined;
  void listen<T>(channel, handler)
    .then((un) => {
      if (cancelled) {
        un();
      } else {
        unlisten = un;
      }
    })
    .catch((e) => {
      console.warn(`[listenScoped] subscribe to ${channel} failed:`, e);
    });
  onCleanup(() => {
    cancelled = true;
    unlisten?.();
  });
}

/// Subscribe to an event with manual lifecycle — returns the UnlistenFn the
/// caller stores + calls itself (the non-scoped path: explicit `onCleanup`,
/// fire-and-forget module-level listeners, awaited setup).
export function listenTo<T>(channel: OaEventName, handler: EventCallback<T>): Promise<UnlistenFn> {
  return listen<T>(channel, handler);
}

/// Publish an event with an optional payload.
export function emitEvent<T>(channel: OaEventName, payload?: T): Promise<void> {
  return emit(channel, payload);
}
