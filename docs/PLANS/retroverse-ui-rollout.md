# Retroverse UI rollout — phase plan

**Status:** Plan locked 2026-05-28. Phase A in flight.

**Author:** Operator + Claude (LLM pair).

**Owner-of-decisions:** the operator.

**Reference:** Per-tab designs at
[settings-tab-retroverse.md](settings-tab-retroverse.md),
[play-now-tab-retroverse.md](play-now-tab-retroverse.md),
[discover-tab-retroverse.md](discover-tab-retroverse.md),
[collections-tab-retroverse.md](collections-tab-retroverse.md);
cross-cutting pack mechanism at
[content-packs.md](content-packs.md); operator-supplied
HOME + LIBRARY mockups at
[features/per-system-ui/assets/](../features/per-system-ui/assets/).

---

## 1. TL;DR

Roll the Retroverse-style UI out tab-by-tab behind a feature flag
(`experimental_retroverse_ui`) so the existing UI keeps working
through the transition. Three phases:

- **Phase A — foundation (invisible to operator with flag OFF).**
  Feature flag, route model extension, RightDetailPanel lift from
  GameInfoModal, play_time_secs increment hooks.
- **Phase B — first visible payoff (LIBRARY).** Three-pane
  RetroverseShell + top-tab strip + re-skinned LIBRARY page. The
  operator can toggle the flag and see the new UI for the first
  time.
- **Phase C — fill out the tabs (one per slice, any order).**
  SETTINGS / HOME / COLLECTIONS / PLAY NOW / DISCOVER. Each ships
  as a separate branch under the flag.

Nothing here requires a big-bang rewrite. Foundation is mostly
already in place (Shell + TopToolbar + LeftSidebar + RightSidebar
primitives, controller-nav v2, per-system theming, media taxonomy,
window prefs persistence) — the new design slots into the existing
3-pane shape with different content per tab.

---

## 2. Foundation already in place ✅

Confirmed by code-grep, not assumed:

- **Three-pane layout primitives** at `frontend/src/layout/`
  (`Shell.tsx`, `TopToolbar.tsx`, `LeftSidebar.tsx`,
  `RightSidebar.tsx`). Same shape Retroverse uses.
- **`TopToolbar` is a generic three-zone slot** (left / center /
  right) — re-skinning App.tsx's usage touches App.tsx, not
  TopToolbar.
- **View routing pattern** — `currentView` signal +
  `<Match when={currentView().kind === ...}>` blocks in
  `App.tsx`. Extending to handle tab kinds is incremental.
- **Per-system theming + asset registry** —
  `frontend/src/themes/systemUIConfigs.ts` + `registry.ts` from
  per-system-ui Stage 1.
- **Controller-nav v2 polish** — DOM-query focus groups,
  identity-tracked focus, DPad / shoulder / back-stack.
- **Library DB + media taxonomy + 4-bus audio mixer** — all
  shipped.
- **`play_time_secs` column** exists in `library_db.rs:979`.
  *(But: grep finds no update site — see Phase A Slice 2.)*
- **Window geometry persistence + tile-size slider** —
  ui-polish work.

---

## 3. Hard blockers identified 🚧

Three things must exist before any Retroverse tab can render
meaningfully:

1. **Top-level page model.** Today `SidebarView` has
   `kind: "all" | "library-manager" | "cores" | viewForSystem(...)`.
   Retroverse needs HOME / LIBRARY / COLLECTIONS / PLAY NOW /
   DISCOVER / SETTINGS as peers.
2. **Feature flag.** Tab-by-tab rollout needs
   `experimental_retroverse_ui` so existing UI keeps working.
3. **Persistent right-detail pane.** Today `GameInfoModal` is a
   modal. Retroverse keeps a focused-game detail panel always
   visible in LIBRARY / PLAY NOW / COLLECTIONS / DISCOVER.

Plus one data gap:

4. **`play_time_secs` is never incremented.** Schema slot exists,
   no update site. Foundational for half the new panels (every
   focused-game detail shows "Played: 8h 12m").

---

## 4. Phase A — foundation (this is the in-flight branch)

Branch: `feat/retroverse-ui-phase-a`. Operator visibly sees
nothing change with flag OFF. With flag ON, no new tabs exist
yet — flag is wired but inert until Phase B.

### Slice 1 — feature flag

- Add `experimental_retroverse_ui: bool` to `SystemSettings` (or
  `Prefs`, whichever holds OA-wide toggles today). Default OFF.
- Frontend: `isRetroverseUiEnabled()` accessor at
  `frontend/src/lib/featureFlags.ts` (or wherever
  per-system-ui's master toggle lives).
- Settings → Display → "Experimental" sub-section gets a
  `Retroverse UI (experimental)` toggle with explanatory text.

Out of scope: no UI swap behavior. Flag is wired but no
consumer reads it yet.

### Slice 2 — play_time_secs increment hooks

- Identify launch + exit boundaries in `apps/oa-shell/src/main.rs`
  (or wherever the emulator session lifecycle lives — likely
  `launch_game` / session exit).
- On launch: record `launched_at: Instant`.
- On exit: compute `delta = now - launched_at`, update the
  library_db row via the existing query pattern.
- Surface in `GameInfoModal` (or wherever play time is shown
  today — likely placeholder right now) so the data is
  immediately visible.

Out of scope: no UI swap. Just the data plumbing.

### Slice 3 — `RightDetailPanel` lift

- Refactor `GameInfoModal.tsx` so the modal's *contents* split
  into a `RightDetailPanel` component that's renderable in two
  modes:
  - **Modal** (current behavior) — wrapped in the existing
    Dialog primitive.
  - **Panel** (new) — bare component, suitable for embedding in
    a 3-pane shell.
- All existing consumers of `GameInfoModal` keep working
  unchanged. Old code path stays.
- Export `RightDetailPanel` from `frontend/src/components/`
  so Phase B's LIBRARY tab can import it.

Out of scope: no consumer of the panel form yet. Just the
refactor.

### Slice 4 — route model extension

- Add `currentRoute` signal at
  `frontend/src/routing/currentRoute.ts` (or similar). Possible
  values: `"home" | "library" | "collections" | "play-now" |
  "discover" | "settings"`. Default `"library"` (matches today's
  default landing).
- `SidebarView` stays as-is — it now represents the *content* of
  the LIBRARY tab specifically, not the whole shell. The other
  tabs will own their own state when they land.
- App.tsx adds a `<Show when={isRetroverseUiEnabled()}>` wrapper
  around the new tab-aware shell, falling through to the
  existing layout when flag is OFF.
- Add a debug-only keybinding (e.g. Ctrl+Shift+R) to cycle
  `currentRoute` for testing without UI yet.

Out of scope: no tab UI yet. Just the state model + a debug
cycle path.

### Phase A success criteria

- `cargo test` workspace stays green.
- Existing UI is byte-identical to operator with flag OFF.
- Flag toggle in Settings → Display → Experimental works.
- Toggling flag ON does nothing visible yet (the wrapper is
  added but empty — Phase B fills it in).
- `play_time_secs` shows non-zero after launching a game and
  exiting.
- A printf or DevTools inspection confirms `currentRoute`
  cycles through the six values via the debug keybinding.

Push branch when all four slices ship green. Operator validates
the four points above before merge.

---

## 5. Phase B — first visible payoff (LIBRARY)

Branch: `feat/retroverse-ui-phase-b-library`. Cut from main
after Phase A merges.

### Slice 5 — `RetroverseShell` layout

- New `frontend/src/layout/retroverse/RetroverseShell.tsx` —
  three-pane shell (left sidebar / center / right detail), each
  zone a slot the consuming page fills.
- Owns the top-tab strip (HOME / LIBRARY / COLLECTIONS / PLAY
  NOW / DISCOVER / SETTINGS) in the TopToolbar's center slot.
- Routes the active tab to the matching page component.

### Slice 6 — `LibraryPage` tab (Retroverse-styled)

- New `frontend/src/routes/retroverse/LibraryPage.tsx`.
- Re-uses existing `VirtualLibraryGrid` for the tile grid (one
  small change: add the mini system-label header per tile).
- Left sidebar = existing systems-filter list (re-skinned).
- Right pane = `RightDetailPanel` from Phase A's Slice 3,
  always-visible (no more modal in this code path).
- Footer hint bar via existing `HintBar` primitive.

### Slice 7 — footer hint bar coverage

- Standardize hint-bar contents for LIBRARY per
  [library-default-mockup design](../features/per-system-ui/assets/README.md).
- Ⓐ PLAY / Ⓑ BACK / Ⓧ SEARCH / Ⓨ FILTERS / VIEW / RS CHANGE SYSTEM.

### Phase B success criteria

- With flag OFF, existing UI byte-identical.
- With flag ON, operator lands on LIBRARY tab styled per the
  Retroverse mockup.
- LIBRARY tab is fully usable: search, filter, sort, view
  toggle, launch a game, focus a game and see persistent
  right-side detail.
- Other tabs are placeholder ("not built yet") — operator can
  click them and see a stub but no error.
- `cargo test` workspace stays green.

Push for operator validation; merge on thumbs-up.

---

## 6. Phase C — fill out the tabs

One branch per tab, in this recommended order. Each ships
independently behind the same flag; operator validates each
before merge.

### C1 — SETTINGS (simplest, all data exists)

Port the four existing `SettingsDialogs` (Display / Audio /
Gameplay / Shaders) into the persistent-tab category shape per
[settings-tab-retroverse.md](settings-tab-retroverse.md). Add
the new categories with stub content (Themes / Controller nav /
Per-system UI promoted as their own categories). Live-preview
pane is sample tile / sample audio meter / sample shader. Per-
system group reuses `SystemSettingsDialog` body inside the
middle pane.

### C2 — HOME (highest content cost — needs hero art + blurbs)

Net-new components: per-system hero panel, QUICK LAUNCH strip,
SYSTEM STATUS gauges (CPU/RAM/Storage via `sysinfo` crate),
RECENTLY PLAYED carousel. Per-system hero art + blurb data
must land first — likely a parallel content workstream
(operator-curated or AI-generated baseline). Procedural
gradient header as fallback for systems without hero art.

### C3 — COLLECTIONS (manual-list version)

Per [collections-tab-retroverse.md](collections-tab-retroverse.md).
Manual collections + smart-list built-ins land first; smart-
query AST + curated-pack consumption ship as follow-ups.

### C4 — PLAY NOW (simple-heuristic recommender)

Per [play-now-tab-retroverse.md](play-now-tab-retroverse.md).
Recommendation engine v1: simple heuristic combining
`last_played_at` + `play_time_secs` + favorite-tagged +
mood-rules-lookup. "Why" lines pulled from a template table.
Rails reweight per mood.

### C5 — DISCOVER (empty state first)

Per [discover-tab-retroverse.md](discover-tab-retroverse.md).
Ships visible-but-empty until content-packs infrastructure
lands. The empty-state CTAs point at SETTINGS → Content →
Packs, which itself doesn't exist yet — initial CTA
disabled with a "Coming soon" hint.

### C6 — content-packs infrastructure

Per [content-packs.md](content-packs.md). Its own arc — the
`oa-packs` Rust crate, SETTINGS → Content + Privacy panels,
end-to-end install/update/uninstall flows, sha256 verification,
and the OA Editorial Baseline pack as the first registry entry.

After C6 lands, DISCOVER becomes functional.

---

## 7. Things that can ship piecewise (don't gate rollout) 💚

- **Achievement progress** — placeholder "—/—" until
  RetroAchievements integration lands.
- **Per-game session-length averages** — start with raw
  `play_time_secs` divided by launch count, refine later.
- **"Why" lines in PLAY NOW** — start with template strings
  ("Last played 3 days ago"), refine algorithmically over time.
- **Smart-query collections** — manual lists ship first.
- **Curated pack content** — empty states handle no-pack case.

---

## 8. Flag deprecation

Once all six tabs ship + operator + a few external testers
validate Retroverse UI is the better default, flip the flag's
default to ON. After one release cycle of stable default-ON,
remove the flag + remove the legacy UI code paths. This is the
**end of the rollout** — at that point Retroverse UI is just
"the UI."

Stale legacy code lives in-tree until the flag retirement; do
not delete pre-emptively. The transition window is the
operator's safety net.

---

## 9. Open questions (revisit per-phase)

- **MenuBar coexistence.** Today's MenuBar (Start-to-open from
  controller-nav v2 polish) lives in the same top region the
  new tab strip wants. Coexist (menu bar above tab strip), or
  hide menu bar in Retroverse mode and surface its actions
  via the SETTINGS / Profile chip instead? Default plan: hide
  menu bar in Retroverse mode; revisit if operator misses it.
- **Default tab on entry.** Retroverse boots into LIBRARY today.
  Should it open HOME instead once HOME ships? Default plan:
  LIBRARY first (faster to "I want to play"), revisit after HOME
  ships and operator tries both.
- **Mouse-only chrome.** Some affordances (pin toggle, sidebar-
  hide button) intentionally stay mouse-only per the
  controller-nav v2 polish DECISIONS. They survive into
  Retroverse unchanged.
- **Per-system theming under Retroverse.** Per-system-ui Stage 1
  configs (tile aspect, interaction style, SFX, boot anims,
  background art) apply across all Retroverse tabs without
  modification — they're already consumed by `LibraryTile` +
  `SystemBackground` + `audio_player`. Stage 2 + Stage 3 work
  composes on top of Retroverse.

---

## 10. Status surface

This document is the source of truth for Retroverse rollout
status. After each phase ships:

- Flip the phase's "in flight" → "shipped" in this doc.
- Append a one-line entry to `docs/SESSION_LOG.md`.
- Update `docs/ACTIVE_WORK.md` to reflect what's in flight.
- Per-tab implementation details land in commit messages, not
  here (this doc stays high-level).

Current status (2026-05-28):

- ✅ Designs locked for all six tabs + content-packs plumbing.
- ✅ Phase A merged — branch `feat/retroverse-ui-phase-a` shipped
  in 4 commits (`9cb42fd` flag, `c845bd7` play_time hooks,
  `ac96452` RightDetailPanel lift, `943ce80` route model).
  Operator-validated; merged `--no-ff` to main as `1c4dee7`.
- ✅ Phase B merged — branch `feat/retroverse-ui-phase-b-library`
  shipped in 4 commits (`b7bdb81` shell + routing, `5bb59fa`
  LibraryPage real, `578052d` hints + tab cycling, `d444b4b`
  fullBleed gate fix). Operator-validated; merged `--no-ff` to
  main as `378863a`. Operator can toggle Settings → Display →
  Experimental → Retroverse UI ON and see the new top-toolbar IA
  with a fully-working LIBRARY tab + persistent right-side detail.
- ✅ Phase C1 merged — branch `feat/retroverse-ui-phase-c1-settings`
  shipped in 2 commits (`39c191d` layout, `d27fc5c` real category
  bodies). Operator-validated; merged `--no-ff` to main as
  `0671726`. SETTINGS tab fully working for the 7 implemented
  categories (Display / Audio / Shaders / Gameplay / Controller
  nav / Per-system UI / Experimental). Other 7 categories
  (Themes / Library / Media / Cores / BIOS / Storage / Profile /
  About) show "Coming in a follow-up slice" placeholders pointing
  at the legacy menu-bar surface.
- ✅ Phase C3 merged — branch `feat/retroverse-ui-phase-c3-collections`
  shipped Slice 11a (`96f6570` data layer — favorite + completed
  columns wired end-to-end with LibraryTile heart overlay) +
  Slice 11b (`8b8eadb` CollectionsPage 3-pane with 4 wired smart
  lists + 2 placeholders). Operator-validated; merged `--no-ff`
  to main as `715f639`. Slice 12 (custom-manual collections —
  new SQLite tables + CRUD) deferred to a follow-up branch.
- ✅ Phase C4 merged — branch `feat/retroverse-ui-phase-c4-play-now`
  shipped Slice 14 (`fd8684f`). PlayNowPage with hero card +
  WHY-line generator + 3 rails + mood sidebar. Operator-validated;
  merged `--no-ff` to main as `b2af79e`. 5 of 6 tabs now operator-
  facing.
- ✅ Phase C2 merged — code-first HOME skeleton + controller-nav
  focus-group activation fix. Branch
  `feat/retroverse-ui-phase-c2-home` shipped Slice 16 (`9b47227`
  HOME skeleton + sysinfo dep) + controller-nav fix (`807915b`).
  Operator-validated; merged `--no-ff` to main as `ca4ab04`.
  Followed by HOME v2 operator-supplied mockup redesign merged
  2026-05-28 (`42da52f`, `feat/retroverse-ui-home-v2` →
  `6aa4c15`): dense SYSTEM INFORMATION + TECHNICAL DETAILS +
  SUPPORTED PERIPHERALS + ACHIEVEMENTS right pane (stub data),
  bigger hero with stats grid + popular-cover carousel, Quick
  Launch panel pinned at bottom-left of sidebar, Recently Played
  separate panel.

  All 6 Retroverse tabs are now operator-facing.

  Content gap follow-ups (deferred):
  - Per-system hero art — drop console + fanart files into the
    existing PlatformMedia slots (infra ships; no code work).
  - Per-system specs — replace systemMetadataStubs.ts with a
    real schema sourced from Wikipedia / TheGamesDB / etc. SNES
    + 6 other priority systems are mockup-faithful stubs; rest
    show "—" until populated.
  - Per-system blurbs — currently embedded in
    systemMetadataStubs.ts; could move to a separate
    blurbs.json or stay as part of the specs schema once that
    materializes.
  - RetroAchievements integration — right pane's achievements
    card is a placeholder with stub numbers.

  Controller-nav v2 (operator spec, merged 2026-05-28 `71816bf`):
  - DPad / left-stick LEFT/RIGHT now transfers between sidebar ↔
    center ↔ right pane on 4 of 5 Retroverse pages (HomePage /
    CollectionsPage / PlayNowPage / SettingsPage). UP/DOWN walks
    within a region; L1/R1 still cycles tabs at the shell level.
    Framework gained `onDirection` + `autoActivate` on
    `useDomQueryFocusGroup`. LibraryPage NOT yet ported —
    embedded LeftSidebar + VirtualLibraryGrid carry their own
    shoulder-bumper-neighbour-wired groups consumed by the legacy
    UI too; aligning needs a Retroverse-mode override on those
    component-level groups.

  System Status fix (merged 2026-05-28 `71816bf`):
  - Persistent sysinfo handle so CPU% reads a real delta.
    Section relocated from center pane to bottom of right pane
    as colored gauges (green/amber/red CPU+RAM, inverted for
    storage free).
- ⬜ Phase C5-C6 — DISCOVER + content-packs infra. Pending
  earlier phases.
