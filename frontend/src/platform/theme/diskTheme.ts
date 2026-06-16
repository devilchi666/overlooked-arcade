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
import type { ThemeTokens } from "./tokens";
import type { PerSystemTokens } from "@oa/platform/themes/systemPalettes";

/// The manifest as it exists ON DISK — the full `ThemeManifest` declarative
/// surface MINUS `entry`/`entry_export` (implicit for declarative themes, D45).
export type DiskThemeManifest = Omit<ThemeManifest, "entry" | "entry_export">;

/// One discovered on-disk theme, mirroring the Rust `DiskThemeDescriptor`.
/// `tokens` / `perSystemTokens` are the parsed optional sidecars; `basePath` is
/// the theme's absolute directory (for resolving its own assets later).
export type DiskThemeDescriptor = {
  manifest: DiskThemeManifest;
  tokens?: Partial<ThemeTokens>;
  perSystemTokens?: PerSystemTokens;
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
  };
}
