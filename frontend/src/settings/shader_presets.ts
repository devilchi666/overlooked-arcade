import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

/// Summary view of a TOML preset (built-in or user-provided). Mirrors
/// the Rust `ShaderPresetSummary` returned by `list_shader_presets`.
export type ShaderPresetEntry = {
  name: string;
  displayName: string;
  description: string;
  /// Renderer pipeline backing this preset — `plain` / `scanlines` /
  /// `crt-lite` / `phosphor`. Surfaced for future UI affordances (e.g.
  /// only show the bloom slider when base === "phosphor").
  base: string;
};

/// Hardcoded fallback used until `list_shader_presets` returns. Matches
/// the four built-in .preset.toml files we ship — keeps the dropdown
/// non-empty during the brief window between app mount and the Tauri
/// roundtrip.
const FALLBACK_PRESETS: ShaderPresetEntry[] = [
  { name: "plain", displayName: "Plain", description: "Nearest-neighbour blit, no post-processing.", base: "plain" },
  { name: "scanlines", displayName: "Scanlines", description: "Alternate-row darken at the source-pixel rate.", base: "scanlines" },
  { name: "crt-lite", displayName: "CRT-Lite", description: "Scanlines + radial vignette + saturation lift.", base: "crt-lite" },
  { name: "phosphor", displayName: "Phosphor (soft bloom)", description: "Separable Gaussian blur composited with the source.", base: "phosphor" },
];

const [presets, setPresets] = createSignal<ShaderPresetEntry[]>(FALLBACK_PRESETS);

let loadOnce: Promise<ShaderPresetEntry[]> | null = null;

/// Fetch the live registry from Rust. Idempotent — repeated calls reuse
/// the same in-flight promise. Call once at app mount; the resulting
/// signal updates `presets()` for every subscribed component.
export function loadShaderPresets(): Promise<ShaderPresetEntry[]> {
  if (loadOnce) return loadOnce;
  loadOnce = invoke<ShaderPresetEntry[]>("list_shader_presets")
    .then((list) => {
      if (Array.isArray(list) && list.length > 0) {
        setPresets(list);
      }
      return list ?? FALLBACK_PRESETS;
    })
    .catch((e) => {
      console.warn("[shader_presets] list_shader_presets failed:", e);
      return FALLBACK_PRESETS;
    });
  return loadOnce;
}

/// Slice D — handler for the `oa://shader-presets-changed` event payload,
/// which is the freshly-loaded summary list. Updates the signal directly
/// so dropdowns refresh without needing another Tauri roundtrip. Also
/// resets the load-once cache so a subsequent `loadShaderPresets()` call
/// would re-fetch (defensive — in practice the event payload is already
/// the authoritative answer).
export function applyShaderPresetsUpdate(list: ShaderPresetEntry[]): void {
  if (Array.isArray(list) && list.length > 0) {
    setPresets(list);
    loadOnce = Promise.resolve(list);
  }
}

/// Read-only accessor used by dropdowns. Reactive — components re-render
/// when the registry loads. Returns the fallback list until then.
export function shaderPresets(): ShaderPresetEntry[] {
  return presets();
}

/// Look up display info for a preset name. Falls back to the name itself
/// if the registry hasn't loaded or the name isn't registered.
export function shaderPresetEntry(name: string): ShaderPresetEntry | undefined {
  return presets().find((p) => p.name === name);
}

/// Display label helper. Returns the `displayName` from the registry
/// (the TOML's display_name field), or the raw name if unknown.
export function shaderPresetLabel(name: string): string {
  return shaderPresetEntry(name)?.displayName ?? name;
}
