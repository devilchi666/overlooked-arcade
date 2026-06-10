// Frontend audio service — wraps the Rust audio_player Tauri commands.
//
// Phase 4 of the media-taxonomy plan. The Rust side owns a rodio-backed
// mixer with 4 buses (platform-music, ui-sounds, ceremony, snap-audio);
// the frontend just dispatches file paths over these wrappers. Live
// game audio still rides the dedicated oa-audio crate's PCM pipe —
// don't route emulator output through here.
//
// UX guarantee: every dispatch is "best-effort, silent on miss." If
// the path resolver returns null (no override configured) the
// playback no-ops. If the Rust audio thread can't open an output
// device at startup, every command silently drops. Never blocks the
// UI; never throws on missing files.

import { createSignal, type Accessor } from "solid-js";
import { resolvePlatformMusic, resolveUiSound } from "@oa/platform/api/shellApi";
import { listen } from "@tauri-apps/api/event";
import * as settingsApi from "@oa/platform/api/settingsApi";

export type AudioBus = "platform-music" | "ui-sounds" | "ceremony" | "snap-audio";

/// Reactive "now playing" descriptor for the platform-music bus. Set
/// when `dispatchPlatformMusic` resolves a non-null path; cleared when
/// it resolves null OR when `stopAudio("platform-music")` runs. The
/// HintBar subscribes to this to render a small chip while music
/// plays. Tracks only what we asked the bus to play — if the Rust
/// side fails to open the file the optimistic state still shows the
/// chip; the dispatch warns to console but doesn't unwind here.
export type NowPlaying = {
  systemId: string;
  gameId: string | null;
};
const [nowPlayingSig, setNowPlayingSig] = createSignal<NowPlaying | null>(null);
export const nowPlaying: Accessor<NowPlaying | null> = nowPlayingSig;

/// Payload for the Rust-side `oa://audio-playback-failed` event.
/// `bus` matches the kebab-case AudioBus serde encoding so a direct
/// string compare against AudioBus values works.
type PlaybackFailedPayload = {
  bus: AudioBus;
  reason: string;
};

// Subscribe to playback failures from the Rust audio thread. When a
// Play command can't open / decode / allocate a sink for the
// requested file, the audio thread emits this event with the bus
// that failed. We clear the now-playing signal when the failing bus
// matches platform-music so the HintBar chip doesn't sit there
// indefinitely showing a song that never plays.
//
// The listen() call is fire-and-forget — it returns an unlisten
// function but we want this for the entire app lifetime. Errors are
// non-fatal: the chip just degrades to dispatch-tracking-only.
void listen<PlaybackFailedPayload>("oa://audio-playback-failed", (event) => {
  if (event.payload.bus === "platform-music") {
    setNowPlayingSig(null);
  }
  // ui-sounds / ceremony / snap-audio failures are silent at the
  // chip level (no UI state to clear) — the audio dispatch helpers
  // log the warning and move on.
}).catch((e) => {
  console.warn("[oa-audio] failed-event listener setup failed:", e);
});

/// Discrete UI-sound events the per-system override map keys on.
/// Matches the SystemSettings.ui_sound_* field names + the
/// resolve_ui_sound command's accepted event strings on the Rust side.
export type UiSoundEvent =
  | "click"
  | "navigate"
  | "back"
  | "launch"
  | "error"
  | "scroll-tick"
  // Per-System UI Stage 1 Slice 4: fires when the boot animation
  // starts on a system-entry transition. Resolver looks at the
  // bundled asset at <systemId>/sounds/boot-intro.<ext> only (no
  // SystemSettings override field in v1).
  | "boot-intro";

/// Play a file on the named bus. `looped = true` keeps the source
/// running indefinitely — appropriate for BGM-shaped buses (platform-
/// music, snap-audio); leave false for one-shot UI cues + ceremony.
/// Replaces whatever was on the bus at the time.
export async function playAudio(bus: AudioBus, path: string, looped = false): Promise<void> {
  try {
    await settingsApi.playAudio(bus, path, looped);
  } catch (e) {
    console.warn("[oa-audio] play failed:", e);
  }
}

export async function stopAudio(bus: AudioBus): Promise<void> {
  try {
    await settingsApi.stopAudio(bus);
  } catch (e) {
    console.warn("[oa-audio] stop failed:", e);
  }
  if (bus === "platform-music") setNowPlayingSig(null);
}

export async function setAudioVolume(bus: AudioBus, gain: number): Promise<void> {
  try {
    await settingsApi.setAudioVolume(bus, gain);
  } catch (e) {
    console.warn("[oa-audio] set volume failed:", e);
  }
}

/// Resolve + play the platform music for a (system, optional game)
/// pair. Cascade: per-game override → per-system override → silence.
/// When the resolver returns null, stops the platform-music bus so
/// any prior track from a different focus doesn't keep playing.
export async function dispatchPlatformMusic(
  systemId: string,
  gameId: string | null,
): Promise<void> {
  try {
    const path = await resolvePlatformMusic(systemId, gameId);
    if (path) {
      await playAudio("platform-music", path, true);
      setNowPlayingSig({ systemId, gameId });
    } else {
      await stopAudio("platform-music");
      // stopAudio above already clears the signal — no double-write.
    }
  } catch (e) {
    console.warn("[oa-audio] dispatch platform music failed:", e);
  }
}

/// Resolve + play a per-system UI-sound event (fire-and-forget). When
/// no override is configured for `event` on `systemId`, the dispatch
/// silently no-ops — desktop UI stays silent by default unless the
/// operator opts in.
export async function dispatchUiSound(
  systemId: string,
  event: UiSoundEvent,
): Promise<void> {
  try {
    const path = await resolveUiSound(systemId, event);
    if (path) {
      await playAudio("ui-sounds", path, false);
    }
  } catch (e) {
    console.warn("[oa-audio] dispatch ui sound failed:", e);
  }
}
