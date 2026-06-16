// Bare (declarative) — the P.1 S2 DOGFOOD: the `bare` theme re-expressed as
// PURE DATA, rendered by the built-in `DeclarativeShell` (Theming ARC 2 "P";
// decisions D45/D47/PD1/PD3).
//
// This proves the whole zero-code path: a `DiskThemeDescriptor` (the exact JSON
// shape the Rust `oa_themes_list_disk` command returns) → `diskThemeToPackage`
// → a `ThemePackage` whose entry is `DeclarativeShell` → a working browse +
// launch shell, selectable from Settings → Appearance, with NO theme-specific
// component code. It sits beside the hand-coded `bare` so the two can be A/B'd:
// they should look + behave the same (the S2 "visually equivalent" acceptance).
//
// Why a built-in (not an on-disk file) in S2: it exercises the descriptor →
// package → render path NOW, decoupled from the disk-loading + registry-merge
// wiring (that's P.1 S3, which swaps the SOURCE of this descriptor from this
// inline object to `oa_themes_list_disk` — the render path is identical). The
// descriptor mirrors `bare`'s manifest, plus the one thing the generic shell
// needs to render a LIST rather than the engine-default grid:
// `views["game-browse"].layout = "list"`.

import { diskThemeToPackage, type DiskThemeDescriptor } from "@oa/platform/theme/diskTheme";
import type { ThemePackage } from "@oa/platform/theme/types";

const BARE_DECLARATIVE_DESC: DiskThemeDescriptor = {
  manifest: {
    id: "bare-declarative",
    name: "Bare (declarative)",
    version: "1.0.0",
    schema_version: 1,
    oa_version: "^0.x",
    default_route: "library",
    routes: ["library"],
    context_slots: ["library"],
    required_engine_capabilities: [],
    reserves_corner: "top-right",
    surfaces: ["main"],
    // PlayStation glyphs — same identity demo as hand-coded `bare`.
    glyph_set: "playstation",
    // The generic shell defaults to the engine `grid`; declare `list` so the
    // dogfood matches `bare`'s vertical list.
    views: { "game-browse": { layout: "list" } },
    // Declared appearance option; the engine Appearance panel renders it and the
    // DeclarativeShell honors it (recognized `compactRows` → list density).
    settings_schema: [
      {
        key: "compactRows",
        type: "toggle",
        label: "Compact rows",
        hint: "Tighter list spacing",
        default: false,
      },
    ],
  },
  // Same scoped per-system palette demo as `bare`: NES → cyan, PSX → magenta,
  // proving the override is theme-mount-scoped (engine territory keeps baseline).
  perSystemTokens: {
    nes: { accent: "oklch(0.72 0.16 195)" },
    psx: { accent: "oklch(0.65 0.22 340)" },
  },
  // No bundled assets → no `basePath` to resolve against.
  basePath: "",
};

export const bareDeclarative: ThemePackage = diskThemeToPackage(BARE_DECLARATIVE_DESC);
