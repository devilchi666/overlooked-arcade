// Disk-theme bridge — turns an on-disk `.oatheme` descriptor (the JSON the Rust
// `oa_themes_list_disk` command returns) into a runtime `ThemePackage` whose
// entry is the built-in `DeclarativeShell` (Theming ARC 2 "P" P.1 S2;
// decisions D45/D47/PD1/PD3).
//
// A declarative theme ships NO code, so the on-disk descriptor carries no
// `entry`/`entry_export` (the Rust `DiskThemeManifest` omits them — D45). This
// mapper supplies them implicitly: every declarative theme renders through the
// one `DeclarativeShell`. The synthetic `entry`/`entry_export` strings exist
// only to satisfy the shared `ThemeManifest` contract + `validateTheme()`'s
// "required non-empty string" check — nothing loads a module from them
// (custom-JS loading is the deferred P.2, D45).
//
// The shape here mirrors `apps/oa-shell/src/theme_loader.rs::DiskThemeDescriptor`
// 1:1 (manifest keys snake_case; token keys camelCase; `basePath`), so the JSON
// the command returns deserializes straight onto `DiskThemeDescriptor` with no
// transform. S3 wires App.tsx to call the command, map each result through here,
// and merge into the registry alongside `BUILTIN_THEMES` (where `validateTheme`
// runs on them, same as builtins). S2 only needs the mapper + the
// `DeclarativeShell` it points at; the dogfood (`themes/declarative-bare/`)
// proves the whole data → package → render path with an inline descriptor.

import DeclarativeShell from "./declarativeShell";
import type { ThemeManifest } from "./manifest";
import type { ThemePackage } from "./types";
import type { ThemeTokens, ThemeMotionTokens } from "./tokens";
import type { PerSystemTokens } from "@oa/platform/themes/systemPalettes";

/// The manifest as it exists ON DISK — the full `ThemeManifest` declarative
/// surface MINUS `entry`/`entry_export` (implicit for declarative themes, D45).
/// The ARC 3 `motion` field rides through here for free (it's part of
/// `ThemeManifest`), so a disk theme declares `[motion.view_transition]` in
/// `theme.toml` and the Rust loader carries it across (D52).
export type DiskThemeManifest = Omit<ThemeManifest, "entry" | "entry_export">;

/// One discovered on-disk theme, mirroring the Rust `DiskThemeDescriptor`.
/// `tokens` / `perSystemTokens` / `motionTokens` are the parsed optional
/// sidecars; `basePath` is the theme's absolute directory (for resolving its
/// own assets later).
export type DiskThemeDescriptor = {
  manifest: DiskThemeManifest;
  tokens?: Partial<ThemeTokens>;
  perSystemTokens?: PerSystemTokens;
  /// Motion-token overrides (ARC 3 M1) parsed from `tokens.toml`'s motion keys.
  motionTokens?: Partial<ThemeMotionTokens>;
  basePath: string;
};

/// Synthetic `entry`/`entry_export` for a declarative theme. Non-empty so the
/// shared `ThemeManifest` contract + `validateTheme()` are satisfied; never
/// dereferenced (the entry component is always `DeclarativeShell`).
const DECLARATIVE_ENTRY = "<declarative-shell>";
const DECLARATIVE_ENTRY_EXPORT = "DeclarativeShell";

/// Map an on-disk descriptor to a runtime `ThemePackage` rendered by the
/// built-in `DeclarativeShell`. Pure — no I/O, no DOM. The caller (S3 registry
/// merge) runs `validateTheme()` on the result, same as a built-in theme.
export function diskThemeToPackage(desc: DiskThemeDescriptor): ThemePackage {
  return {
    manifest: {
      ...desc.manifest,
      entry: DECLARATIVE_ENTRY,
      entry_export: DECLARATIVE_ENTRY_EXPORT,
    },
    entry: DeclarativeShell,
    tokens: desc.tokens,
    perSystemTokens: desc.perSystemTokens,
    motionTokens: desc.motionTokens,
  };
}

/// Merge discovered disk themes with the built-in set for registration
/// (P.1 S3). Built-ins come first (so the default stays index 0) and WIN any id
/// collision — a community disk theme cannot shadow or duplicate a built-in id
/// (the bundled default must remain a guaranteed, un-overridable fallback floor,
/// D44). Skipped collisions are logged. `validateTheme` still runs on the result
/// at registration, so an invalid disk theme is excluded there, same as a
/// built-in. Pure — takes the built-in list as a param so this stays in
/// platform/ without importing themes/ (platform ↛ theme).
export function mergeDiskThemes(
  builtins: ThemePackage[],
  descriptors: DiskThemeDescriptor[],
): ThemePackage[] {
  const builtinIds = new Set(builtins.map((b) => b.manifest.id));
  const diskPackages: ThemePackage[] = [];
  for (const desc of descriptors) {
    if (builtinIds.has(desc.manifest.id)) {
      console.warn(
        `[oa-theme] disk theme "${desc.manifest.id}" shadows a built-in id; ignoring the disk copy`,
      );
      continue;
    }
    diskPackages.push(diskThemeToPackage(desc));
  }
  return [...builtins, ...diskPackages];
}
