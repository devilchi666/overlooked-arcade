# UI Polish Plan — Concrete Execution Sheet

> **STATUS: ✅ FULLY SHIPPED 2026-05-22.** Phases A (cleanup + renames), B+C (Dialog primitive + SettingRow), D (drawer shrink + 7 Game dialogs), E (CLI stub + chromeVisible) all landed. Menu-bar architecture (the Phase 0 prereq for `../kiosk-shell/KIOSK_PLAN.md`) operationalized via the dialog refactor. This file is kept as historical reference.

**Status:** Ready to execute. Synthesized 2026-05-22 from six parallel codebase audits.

**Purpose:** Eliminate duplicate settings surfaces, polish the Dialog primitive, canonicalize `SettingRow`, and shrink the per-game drawer to a focused Properties dialog. **This is Phase 0 of `../kiosk-shell/KIOSK_PLAN.md`** — the prereq before any kiosk shell work.

**Companion docs:**
- Sibling `UI_AUDIT.md` (2026-05-18) — original inventory; **stale on several points** (see §0 below).
- Sibling `UI_MENU_BAR_PLAN.md` (2026-05-18) — original menu-bar roadmap. ~70% shipped; this plan finishes it.
- `../kiosk-shell/KIOSK_PLAN.md` (2026-05-22) — long-term kiosk design.

---

## 0. Surprising findings from the audit (correct your mental model first)

1. **`frontend/src/components/PerSystemSettingsPage.tsx` does not exist on disk.** The per-system migration to `SystemDialogs.tsx` already shipped. Wiring is correct in `SystemContextMenu.tsx`, `GridControls.tsx` (per-system ⚙ gone), `SystemHeader.tsx` (quick-action chips gone), and `App.tsx` System ▾ menu. **Only stale doc-comments remain to clean up.**
2. **`frontend/src/components/SettingsPage.tsx` already reduced to 2 tabs** (`Library`, `Game media`). The 7-tab claim in `UI_AUDIT.md` §4.1 is stale. The right move is **rename to `LibraryManagerPage`** (the menu-bar plan's surviving full page) — content stays, route key + label change.
3. **`frontend/src/components/PerGameSettingsDrawer.tsx` has 10 tabs, not 11.** Audio was already removed. Cheats is fully functional (CRUD + 4-stage cheat search). ROM patch is fully wired to `pick_patch_file`. None of the audit's "placeholders" in this file are actually placeholders anymore.
4. **Presentation-mode plumbing is in unusually good shape** — `PresentationMode` lives at `frontend/src/layout/state.ts:12-24`, mirrors to `body[data-presentation="..."]` via CSS cascade, has 4 reads + 1 write outside the store. Minimal polish-time work.
5. **Three duplicate `SELECT_CLASS` constants** drift between `SettingsDialogs.tsx:32`, `SystemDialogs.tsx:101`, and `SettingsPage.tsx:75`. SystemDialogs uses system-accent focus ring; the others use ink-dim. Canonicalize via SettingRow's new built-in `select` control.

---

## 1. Phase A — Cleanup (one PR, low risk, immediate "less clutter" win)

Pure-deletion + rename work. No new components, no API changes.

### A.1 Rename `SettingsPage` → `LibraryManagerPage`

**`frontend/src/components/SettingsPage.tsx`:** rename file to `LibraryManagerPage.tsx`.
- L234: `const SettingsPage: Component<Props>` → `const LibraryManagerPage`
- L238: `"oa.settings.activeTab"` → `"oa.library.activeTab"` (accept that operator's last-tab preference resets once)
- L394, L539: `"SettingsPage: …"` warn prefixes → `"LibraryManagerPage: …"`
- L714-723: delete unused `moveRegion` function + its `void moveRegion;` reference at L324 (already commented as unused)
- L794: heading "Settings" → "Library Manager"
- L1390: `export default SettingsPage` → `export default LibraryManagerPage`

### A.2 Rename `SidebarView` discriminant `"settings"` → `"library-manager"`

**`frontend/src/layout/LeftSidebar.tsx`:**
- L13: `| { kind: "settings" }` → `| { kind: "library-manager" }`
- L6-9: rewrite `SidebarView` doc-comment — drops the "per-system settings page deep-link" reference (page no longer exists).
- L23-26: rewrite `Props.onSystemContext` doc-comment (`SystemHeader` ⚙ button referenced is gone).
- **L243-283: delete the entire bottom `Cores` and `Settings` buttons.** `UI_MENU_BAR_PLAN.md` "What gets deleted" §1.2 calls for these; they were never deleted.

**`frontend/src/App.tsx`:**
- L13: `import SettingsPage from …` → `import LibraryManagerPage from "./components/LibraryManagerPage"`
- L245: `setCurrentView({ kind: "settings" })` → `"library-manager"`
- L424: `currentView().kind !== "settings"` → `"library-manager"`
- L466: `currentView().kind === "settings"` → `"library-manager"`
- L1564: `<Match when={currentView().kind === "settings"}>` → `"library-manager"`
- L1566: `<SettingsPage …>` → `<LibraryManagerPage …>`

**`frontend/src/components/LibraryView.tsx:74`:** `case "settings": return "Settings";` → `case "library-manager": return "Library Manager";`

**`frontend/src/library/filter.ts:9, 26`:** verify `SidebarView` import compiles after rename; the filtering pipeline reads this type.

### A.3 Strip stale `PerSystemSettingsPage` references (5 doc-comment sites)

Cosmetic, no behavior change:
- `frontend/src/components/CoresPage.tsx:72` — remove the "same shape as SettingsPage / PerSystemSettingsPage" reference.
- `frontend/src/components/PerGameSettingsDrawer.tsx:48` — update comment that references "PerSystemSettingsPage" as the tier-2 surface → point at `SystemDialogs.tsx`.
- `frontend/src/components/PerGameSettingsDrawer.tsx:1127` — comment "in PerSystemSettingsPage. Launch-path resolution…" → repoint to `SystemDialogs.tsx`.
- `frontend/src/components/SystemBindingsEditor.tsx:26` — "PerSystemSettingsPage Input tab" → "wrapped by SystemBindingsDialog".

### A.4 Update sibling `UI_AUDIT.md` staleness

Add a status header at the top of the file:
> **Status as of 2026-05-22:** Several specifics in this audit have shipped or drifted. See `docs/UI_POLISH_PLAN.md` §0 for the current state. The structural-IA findings (overlap matrix, orphaned features) remain valid as design context.

Do not rewrite the audit; it's a historical reference. The Polish Plan is the authoritative current state.

### A.5 Acceptance criteria for Phase A

- `cd frontend && npm run tsc -- --noEmit` clean.
- Manual visual check: opening Library menu → "Library Manager…" lands on the renamed page. Sidebar has no `Cores` / `Settings` bottom buttons. Right-click a system → "System settings…" still opens `SystemSettingsDialog`. `Esc` during gameplay still opens Quick Settings unless on the Library Manager / Cores page (the gate logic still works).
- `cargo test` should be unaffected (frontend-only change).

---

## 2. Phase B — Dialog primitive polish (one PR with Phase C; they co-evolve)

Touches `frontend/src/layout/Dialog.tsx` plus DisplayDialog as the reference migration. Additive API; no breaking changes.

### B.1 New size scale

`frontend/src/layout/Dialog.tsx`:
```ts
export type DialogSize = "sm" | "md" | "lg" | "xl" | "2xl";

const WIDTH_CLASS: Record<DialogSize, string> = {
  sm:  "max-w-md",    // ~448px — single field / confirm
  md:  "max-w-xl",    // ~576px — default, most settings (was max-w-md)
  lg:  "max-w-3xl",   // ~768px — sectioned forms (was max-w-2xl)
  xl:  "max-w-4xl",   // ~896px — content-rich (Display, Bindings, Properties)
  "2xl": "max-w-5xl", // ~1024px — Properties drawer-replacement, multi-column only
};
```

Old `sm/md/lg` keep their names and widen under the hood; all existing call sites benefit automatically. Audit AudioDialog if it looks oversized at the new `md` — if so, pin it `size="sm"` explicitly (one-character change).

### B.2 New `<DialogSection>` component

Add to `frontend/src/layout/Dialog.tsx` (or extract `DialogSection.tsx` if preferred):

```ts
type DialogSectionProps = {
  title: string;
  description?: string;
  collapsible?: boolean;
  defaultCollapsed?: boolean;
  id?: string;                  // anchor for future nav rail
  children: JSX.Element;
};
```

Layout: `border-t border-white/5 pt-5 first:border-0 first:pt-0`. Header is `text-sm font-semibold` title + optional `text-xs leading-relaxed text-(--color-oa-ink-dim)` description. Rows inside use `gap-4`.

### B.3 Type ramp + spacing updates

| Surface | Current | New |
|---|---|---|
| Dialog title | `text-base font-semibold` (16px) | `text-lg font-semibold` (18px) |
| Dialog subtitle | `text-[0.6rem] uppercase tracking-[0.3em]` | `text-xs text-(--color-oa-ink-dim)` (drop uppercase) |
| Section title | n/a | `text-sm font-semibold text-(--color-oa-ink)` |
| Field label | `text-xs text-(--color-oa-ink-dim)` | `text-sm font-medium text-(--color-oa-ink)` |
| Field hint | `text-[0.65rem] uppercase tracking-widest` | `text-xs leading-relaxed text-(--color-oa-ink-dim)` (drop uppercase) |
| Body padding | `px-5 py-4` | `px-6 py-5` |
| Header padding | `px-5 py-3` | `px-6 py-4` |
| Row gap | `gap-3` | `gap-4` |
| Header accent tint | `bg-(--color-system-accent)/10` | `bg-(--color-system-accent)/15` |
| Header bottom border | `border-white/5` | `border-white/10` |
| Close button glyph | unicode `✕` | SVG ✕ at 14px (consistent across platforms) |

### B.4 Defer `<DialogNavRail>` (left-rail nav for multi-section dialogs)

Not v1. Add `id` prop to `DialogSection` now so the anchor mechanism exists; build the rail when a consolidated Properties dialog has 6+ sections.

### B.5 Migrate `DisplayDialog` as the reference

`frontend/src/components/SettingsDialogs.tsx` L45-143 → wrap in `<DialogSection>` groupings:
- Section "Scaling" — Scaling mode
- Section "Window" — Window mode, Monitor
- Section "Run-ahead" — Run-ahead frames (with description prose instead of all-caps hint)

Bump dialog `size="md"` → `size="xl"`. After migration, the run-ahead description prose lives inside the row, not below it as a footnote.

### B.6 Acceptance criteria for Phase B (judged together with Phase C)

- `tsc --noEmit` clean.
- Manual visual check: DisplayDialog opens at ~896px wide on 1080p; sections are clearly separated; type hierarchy reads cleanly. All other dialogs render normally (just slightly wider for the redefined `md` / `lg`).
- Confirm AudioDialog hasn't become awkwardly empty at the new `md`; if so, pin to `sm`.

---

## 3. Phase C — `SettingRow` canonicalization (paired with Phase B)

Extends the existing primitive so every settings row across every surface uses one component.

### C.1 Extended API for `frontend/src/components/SettingRow.tsx`

```ts
type InheritScope = "oa-default" | "per-system" | "per-game";
type SelectOption = { value: string; label: string; disabled?: boolean };

type SettingRowProps = {
  label: string;
  hint?: string;                          // small strap, kept for short notes
  description?: string | JSX.Element;     // prose under the control (NEW)
  inherited?: {                            // typed (NEW — replaces inheritedValue/inheritedFrom)
    value: string;
    from: InheritScope | string;
  } | null;
  overridden: boolean;
  disabled?: boolean;                     // NEW — dims label + control + chip together
  onReset?: () => void;                   // NEW — renders Reset chip when overridden

  // Built-in controls (mutually exclusive with children)
  select?: {
    value: string;
    options: SelectOption[];
    onChange: (v: string) => void;
    placeholder?: string;
    tone?: "oa" | "system";               // focus ring color
  };
  slider?: {
    min: number; max: number; step: number;
    value: number;
    format?: (v: number) => string;
    onInput: (v: number) => void;
  };
  toggle?: {
    checked: boolean;
    onChange: (v: boolean) => void;
  };
  children?: JSX.Element;                 // escape hatch — custom widgets only
};
```

Existing `inheritedValue` / `inheritedFrom` props deprecated but kept as a passthrough for backward compatibility during migration. Remove after all sites migrated.

### C.2 Delete duplicate `SELECT_CLASS` constants

After `SettingRow.select` ships:
- `frontend/src/components/SettingsDialogs.tsx:32` — delete `SELECT_CLASS`.
- `frontend/src/components/SystemDialogs.tsx:101` — delete `SELECT_CLASS`.
- `frontend/src/components/SettingsPage.tsx:75` (post-rename: `LibraryManagerPage.tsx`) — delete `SELECT_CLASS`.

The built-in `select` styling lives once in `SettingRow.tsx`. Tone (`oa` ink-dim ring vs `system` accent ring) is chosen via the `tone` prop or derived from the dialog's `data-system` context.

### C.3 Migration mapping

| File | Settings rows to migrate | Notes |
|---|---|---|
| `SettingsDialogs.tsx` | ~10 (Display 4, Audio 1, Gameplay 4, Shaders 2) | Pass `inherited={null}` (OA-wide tier). Run-ahead + bloom adopt `slider={…}`. Rewind-enable becomes `toggle={…}`. |
| `SystemDialogs.tsx` | 0 new (already 13 SettingRows) | Bloom slider drops hand-rolled flex-row in favor of `slider={…}` + `onReset={…}`. |
| `LibraryManagerPage.tsx` (renamed from SettingsPage) | ~4 (only-sync-identified toggle, auto-remove-on-delete toggle, revision-tiebreaker select, clear-games-for select) | List/table surfaces (region drag, library folders drag, sidebar systems table, sync button strip, danger zone) stay custom — NOT candidates. |
| `PerGameSettingsDrawer.tsx` | 0 new (already 14) | Bloom slider here also adopts `slider={…}` + `onReset`. |

**Total true migrations:** ~14 settings rows + 2 slider refactors. Plus delete of 3 SELECT_CLASS constants.

### C.4 NOT candidates (don't try to shoehorn)

- Region-priority drag-and-drop list
- Library folders sortable list
- Sidebar systems checkbox table
- Per-system sync button strip with progress bars
- "Kinds to fetch" multi-checkbox pill group
- Danger zone destructive buttons
- PerGame Input tab libretro-device card (banner + per-port grid)
- Milestones draft form (new-record entry, no inheritance semantics)
- Cheats draft form (same)
- `OverscanEditor` / `BezelPicker` (stay as `children` inside SettingRow — they're widgets, not rows)

### C.5 Acceptance criteria for Phase B + C

- `tsc --noEmit` clean.
- Visual: every settings row across every dialog looks the same (label + optional description + control + optional inheritance chip + optional Reset).
- The three SELECT_CLASS constants are deleted; control styling is single-source.
- Inheritance chips still display correctly for per-system and per-game tiers (visual regression check on `SystemSettingsDialog` and `PerGameSettingsDrawer` Display tab).

---

## 4. Phase D — Drawer shrink + Game-menu dialog extraction (biggest PR)

Recommendation: **SHRINK, not retire.** Keep `PerGameSettingsDrawer.tsx` as a consolidated "Properties" dialog at the new `xl` size. Extract 7 of its 10 tabs into focused Game-menu dialogs.

### D.1 Tab disposition

| Tab | Disposition | Target |
|---|---|---|
| Overview | **Keep in Properties** | read-only summary |
| Core | **Keep in Properties** | core override `<select>` + ROM patch picker — fold both into "Properties → Core" |
| Core options | **Extract** | new `GameCoreOptionsDialog` (mirror `SystemCoreOptionsDialog`) |
| Display | **Extract** | new `GameDisplayDialog` (mirror `SystemSettingsDialog(section: "display")`) |
| Input | **Extract** | new `GameInputDialog` — densest tab in the file, wants the room |
| Rewind | **Extract** | new `GameRewindDialog` (3 selects + `RewindLiveStats`) |
| Shaders | **Extract** | new `GameShadersDialog` (mirror OA `ShadersDialog`) |
| Region | **Delete entirely** | Persists but has no runtime effect on any core; duplicates the boxart `RegionPicker` semantically. Backend keeps reading `regionOverride` for future use — just removes the UI. |
| Milestones | **Extract** | new `MilestonesDialog` (xl) — memory editor needs width |
| Cheats | **Extract** | new `CheatsDialog` (xl) — CheatsTab + 4-stage cheat search is cramped at 480px |

`GameDrawerTab` type becomes `PropertiesTab = "overview" | "core"`.

### D.2 New dialogs to create

All mirror existing patterns (`SystemDialogs.tsx` shape for per-system; `SettingsDialogs.tsx` for OA-wide). Use the new `xl` size for Cheats / Milestones / Input / Core options / Display.

Suggested file: `frontend/src/components/GameDialogs.tsx` (one file holds the 7 new dialogs, like `SystemDialogs.tsx` holds three).

### D.3 Flip Game menu items

`frontend/src/App.tsx` L1035-1043 — Game ▾ menu items. Currently:
```tsx
<MenuItem onClick={() => openGameDrawer(entry, "core")}>Core override…</MenuItem>
// ... etc, 7 items
```

After D.3:
```tsx
<MenuItem onClick={() => setGameDialog({ kind: "core-options", target: entry })}>Core options…</MenuItem>
// ... new dialog launch state per item
```

The `Properties…` item keeps calling `openGameDrawer(entry)` (renamed: `openProperties(entry)`) — that's the consolidated dialog.

### D.4 `TileContextMenu` → "Game properties…"

`frontend/src/components/TileContextMenu.tsx:297` — `onOpenProperties(entry)` continues to open the Properties dialog (the slimmed-down drawer-replacement). No signature change. Add separate Cheats / Milestones / Save states items as a follow-up if `TileContextMenu` should mirror the new menu.

### D.5 Properties dialog (drawer replacement)

`frontend/src/components/PerGameSettingsDrawer.tsx` → rename to `GamePropertiesDialog.tsx`. Drop drawer chrome, use `<Dialog size="xl">`. Surviving tabs: Overview, Core. Drop tab strip entirely if both fit on one screen (likely they do at xl).

### D.6 Acceptance criteria for Phase D

- `tsc --noEmit` clean.
- `cargo test` unaffected.
- Manual: Game ▾ → Cheats… opens dedicated CheatsDialog at xl size; cheat search panel + candidate table fit without scrolling laterally. Same for Milestones. Right-click tile → Game properties… opens consolidated Properties at xl size with Overview + Core. All 7 extracted-tab dialogs render and persist correctly through the existing `get_game_overrides` / `set_game_overrides` plumbing.
- No regression in three-tier inheritance display.

---

## 5. Phase E — Presentation-mode plumbing (small, can ship any time)

Two cheap changes that pay off for the future kiosk shell without doing real kiosk work.

### E.1 Add `--kiosk` CLI flag stub

`apps/oa-shell/src/cli.rs`:
- Add `#[arg(long)] pub kiosk: bool` to the args struct.
- In `DirectLaunchConfig`, add `kiosk: bool`.
- At startup, when `--kiosk` is set, **override** the initial presentation mode at runtime only — don't write `presentation.json`. Mirror the precedence used by `OA_SHELL_MODE` in `apps/oa-shell/src/main.rs:2605-2615`.
- Initial override target: `PresentationMode::Cabinet` (the closest existing approximation). When kiosk chrome lands in Phase 1 of `../kiosk-shell/KIOSK_PLAN.md`, add a fourth variant.

### E.2 Extract `chromeVisible()` memo

`frontend/src/App.tsx`:
- Add `const chromeVisible = createMemo(() => !isDirectLaunch() && !gameMode());`
- Replace the existing inline equivalents in the chrome-rendering paths.
- Zero behavior change today. Phase 1 kiosk only has to extend this one line to gate the entire menu bar + toolbar + sidebars off when `presentationMode() === "kiosk-locked"` (or whatever the future variant names).

### E.3 What NOT to do during this phase

- Do not rename `PresentationMode` to `ShellMode` — collides with the existing two-window/single-window concept at `frontend/src/settings/store.ts:14`.
- Do not collapse `presentation.json` into `layout.json` or `shell.json`.
- Do not bake `presentationMode() === "desktop"` checks into leaf components. Gate at chrome boundaries only.
- Do not add a fourth `kiosk` enum variant until kiosk chrome exists (would alias Cabinet today; dead-code branch).
- Do not write a new `kiosk.json` — handled by `presentation.json` for now.

### E.4 Acceptance criteria for Phase E

- `cargo test` clean.
- `oa-shell.exe --kiosk` boots into Cabinet mode (CSS cascade applies; no chrome changes yet because we're not at Phase 1).
- `oa-shell.exe --kiosk` does NOT modify `presentation.json` on disk (override is runtime-only).
- `chromeVisible()` memo is in place; existing behavior unchanged.

---

## 6. Sequencing & PRs

1. **PR 1 — Phase A** (renames + sidebar cleanup + stale comments). Pure-deletion shape. Lowest risk, biggest "less clutter" win. Independently mergeable.
2. **PR 2 — Phase B + Phase C** (Dialog primitive + SettingRow canonicalization + DisplayDialog reference migration). Co-evolved; ship together. Reference for all subsequent dialog work.
3. **PR 3 — Phase D** (drawer shrink + 7 new Game-menu dialogs). Largest PR. Depends on `xl` size from PR 2.
4. **PR 4 — Phase E** (CLI stub + chromeVisible memo). Tiny, can land alongside any of the others or independently.

Per-PR scope discipline: do not touch anything outside the named phase. If you spot a related cleanup, file it as a follow-up note in the PR description, don't fold it in.

---

## 7. Out of scope (deliberately)

- Building kiosk chrome (`../kiosk-shell/KIOSK_PLAN.md` Phase 1+).
- New settings fields.
- Migrating Milestones / Cheats draft-form layouts to SettingRow (they're new-record entry forms, not inheritance-aware settings).
- Rewriting sibling `UI_AUDIT.md` (add the staleness header in A.4; leave content as historical reference).
- Touching emulator-side code beyond the small CLI flag stub in E.1.
- `<DialogNavRail>` scroll-spy left rail (revisit when a 6+ section dialog actually exists).
- Adding a fourth `PresentationMode::Kiosk` variant (waits for kiosk chrome).
- Persistent kiosk profile / auto-launch on boot (parked in `PARKING_LOT.md` 2026-05-20).

---

## 8. Per-PR sign-off checklist

For each PR:
- `cd frontend && npm run tsc -- --noEmit` clean.
- `cargo test` clean.
- Manual visual check at minimum two presentation modes (Desktop + Theater) and one window mode (two-window) — confirms no regressions in CSS cascade or layout state.
- ROADMAP / PARKING_LOT hygiene per `CLAUDE.md`: if the PR closes a tracked item, flip the bullet here AND in the relevant per-core doc.
- Append a SESSION_LOG entry: Shipped / Almost / Next.
