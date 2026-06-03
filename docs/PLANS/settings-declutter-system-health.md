# Settings declutter — System Health hub + Game-media cards

**Status:** Planning locked 2026-06-03. Implementation in flight on
`feat/settings-declutter-system-health`.

**Owner-of-decisions:** the operator. Choices below were settled in
a single back-and-forth on 2026-06-03 (three AskUserQuestion rounds).
Implementation should follow them unless a code-time issue forces a
revisit (in which case: check back in here first).

---

## Why this matters

The SETTINGS → Library page has accumulated four unrelated jobs:

1. "Set up your library" CTA → Import Wizard
2. System Readiness checklist (per-system core/BIOS/art rollup — a big block)
3. Background Jobs — dev test (a developer-only spawn-synthetic-job affordance)
4. The embedded Library Manager with its own Library / Views / Game media tabs

The Game media tab inside that is the worst offender: per-system
rows × 45 systems, each carrying **five action buttons**
(`Sync media / Sync metadata / Clear metadata / Sync hashes /
Identify ROMs`) — ~225 buttons stacked vertically. Plus a separate
header strip with three more (`Platform media… / Import art pack… /
Open folder`), a toggle, a checkbox row, a sortable region list, and
a disk-usage line at the bottom.

The operator wants the page to read calmer and the system-status
information to live somewhere more obvious. This document records
the agreed shape.

---

## Locked design decisions

### 1. System Health is a new top-level SETTINGS category

A new `system-health` category lives in the **SYSTEM** group of the
SETTINGS sidebar. Its body is its own page with an **internal tab
strip** across the top:

```
┌─────────────────────────────────────────────────┐
│ ● Overview │ BIOS │ Cores │ Storage │ Jobs    │
└─────────────────────────────────────────────────┘
```

The four right-hand tabs absorb the existing standalone categories.
**BIOS / Cores / Storage / Background Jobs disappear from the
SETTINGS sidebar** — they live exclusively under System Health now.
Sidebar shrinks from 16 entries to 12.

#### Overview tab

Top half — five status rollup cards in a vertical stack:

| Status | Subject       | Body                              | CTA       |
|--------|---------------|-----------------------------------|-----------|
| ●      | Cores         | "N/M installed · K missing"       | Fix →     |
| ●      | BIOS          | "N/M staged · K missing (list)"   | Resolve → |
| ●      | Readiness     | "N/M systems ready · K incomplete"| See →     |
| ●      | Background    | "Idle / K active · M prefs set"   | Open →    |
| ●      | Storage       | "X GB free · subdir summary"      | Details → |

- Status dot encodes severity: green / amber / red.
- Clicking the CTA switches the top tab strip to that tab. Same
  destination as clicking the tab label directly — the row is just a
  faster path when something needs attention.

Bottom half — the existing `SystemReadinessChecklist` component
renders the per-system grid (lifted from where it lives on
Settings → Library today). The Overview tab is the only home for
the readiness checklist.

#### BIOS / Cores / Storage / Jobs tabs

Each tab body is the existing settings component **lifted verbatim**:
- BIOS → `BiosSettings`
- Cores → `CoresCategorySettings`
- Storage → `StorageSettings`
- Jobs → `BackgroundJobsSettings`

No content changes inside these tabs. The tab strip provides the
navigation; the existing bodies stay as-is.

### 2. Game media — status-first cards in alphabetical order

The per-system row list becomes a **status-first card grid**:

- 3 columns on a normal viewport (auto-fit; falls back to 2 on narrow).
- **Cards sorted alphabetically by display name**. Position is
  predictable regardless of status. (Operator picked this explicitly
  over status-sorted.)
- Each card shows:
  - A theme-accent vertical stripe (`▮`).
  - System display name + game count.
  - Three status rows with check/warning/cross glyphs:
    - `✓/⚠/✗ identified  N/M`
    - `✓/⚠/✗ covers      N/M`
    - `✓/⚠/✗ metadata    N/M`
  - **Two buttons**:
    - `[Freshen]` — runs whatever is incomplete for this system, in
      sequence, via Background Jobs.
    - `[Manage…]` — opens the per-system Manage side panel (see below).

Above the card grid:

- A **Preferences** card hosts the things that are settings (not
  actions):
  - "Only sync identified ROMs" toggle.
  - "Kinds to fetch" checkboxes (Boxart / Snapshots / Title screens).
  - Region priority sortable (collapsed; expandable).
- A small header row with the total system count + incomplete count
  + a single **[Freshen all systems]** CTA on the right.

Below the card grid:

- The existing disk-usage summary line.

### 3. Manage… side panel — granular per-system ops

Clicking `[Manage…]` on a system card opens a **side panel sliding
in from the right** with the five granular ops:

```
┌─ {SYSTEM} · GAME MEDIA OPS ──────────────┐
│ {N} games · last synced {timestamp}       │
│                                            │
│  Identify ROMs           [Run]            │
│    {one-line description + current count} │
│                                            │
│  Sync covers             [Run]            │
│    {description + count}                  │
│                                            │
│  Sync metadata           [Run]            │
│    {description + count}                  │
│                                            │
│  Clear metadata          [Run]            │
│    {description}                          │
│                                            │
│  Hash database           [Refresh]        │
│    {last refreshed timestamp}             │
│                                            │
│                                   [Done]  │
└────────────────────────────────────────────┘
```

- Each op is a one-line description + current count + a single
  `[Run]` button. No naked verbs — every button has context above it.
- Operator can switch active card while the panel is open; the panel
  re-targets the new system without closing.
- All five ops route through the existing handlers (already wired to
  Background Jobs). Operations run in the background; the panel can
  be closed mid-op; progress surfaces on the BackgroundJobsBar.

### 4. Background Jobs — dev test card → Experimental → Dev tools

The "Spawn 30 s test job" / "Spawn 10 s test job" card moves out of
Settings → Library into a new collapsed **Dev tools** sub-section on
the Experimental category page. Collapsed by default; expand to
reveal. Operators who don't need it never see it.

---

## Phases

Each phase is a separate commit on `feat/settings-declutter-system-health`.
Tests + typecheck must pass between phases.

### Phase 1 — System Health scaffolding + sidebar restructure

**Scope:**
- Add `system-health` category id to `SettingsPage.tsx` CATEGORIES.
- Remove `bios` / `cores` / `storage` / `background-jobs` ids from
  CATEGORIES (their bodies move under System Health).
- Create new `frontend/src/routes/retroverse/SystemHealthPage.tsx`
  with the internal tab strip + 5 tab arms (Overview stub + BIOS /
  Cores / Storage / Jobs each rendering the existing component).
- Lift System Readiness card OUT of `LibrarySettings`.
- Overview tab gets a placeholder ("Status rollup — coming Phase 2").

**Files touched:**
- `frontend/src/routes/retroverse/SettingsPage.tsx` (CATEGORIES table
  + Match arm).
- `frontend/src/routes/retroverse/SystemHealthPage.tsx` (new).
- `frontend/src/components/SettingsSections.tsx` (drop the readiness
  block from `LibrarySettings`).

**Exit criteria:**
- `npm run typecheck` silent.
- Sidebar shows System Health entry; BIOS / Cores / Storage / Jobs
  no longer appear as top-level entries.
- Clicking System Health → Overview shows the placeholder; clicking
  any of the 4 absorbed tabs renders the existing body intact.
- Library page no longer shows the System Readiness card.

### Phase 2 — Overview rollup cards + readiness detail

**Scope:**
- New `StatusRollupCard` component (one row in the table above).
  Props: `status` (good/warn/bad) + `title` + `summary` + `cta`.
- Wire 5 cards on the Overview tab. Each fetches its own counts
  via the existing Tauri commands:
  - **Cores** — `list_cores` + system inventory cross-ref.
  - **BIOS** — `get_bios_status` (already used by BiosSettings).
  - **Readiness** — derive from the library entries + the same data
    SystemReadinessChecklist consumes.
  - **Background jobs** — `list_active_jobs` (count) + `get_job_prefs`
    (per-kind opt-out count).
  - **Storage** — `get_system_status` (already used by StorageSettings).
- Clicking a card's CTA switches the active tab inside SystemHealthPage
  (via a setter passed from the parent).
- Below the rollup grid: render `SystemReadinessChecklist` (lifted
  from Library page in Phase 1) against the operator's library
  systems.

**Exit criteria:**
- Five rollup cards render with live data.
- Each card's status pill reflects current state (green when nothing
  needs attention, amber when something is incomplete, red when
  something is missing or broken).
- CTA buttons switch the tab.
- Readiness checklist works exactly as it did on the Library page.

### Phase 3 — Dev test card relocation

**Scope:**
- Add a collapsed "Dev tools" sub-section to `ExperimentalSettings`
  in `SettingsSections.tsx`.
- Move the "Background Jobs — dev test" card into it (Spawn 30 s /
  Spawn 10 s buttons).
- Delete the card from `LibrarySettings`.

**Exit criteria:**
- Library page no longer shows the dev test card.
- Experimental page has a collapsed "Dev tools" disclosure;
  expanding it reveals the two spawn buttons.
- Buttons still call `spawn_test_job` and the bar still surfaces the
  job.

### Phase 4 — Game media preferences hoist + status-first cards

**Scope:**
- Refactor the `activeTab() === "media"` branch in
  `LibraryManagerPage.tsx`.
- Hoist the preferences (Only sync identified + Kinds to fetch +
  Region priority + disk usage region selector) into a top
  **Preferences** card.
- Replace the `<For each={systemIds}>` per-system row loop with a
  3-column card grid (auto-fit / 2-col fallback). Cards sorted
  alphabetically by `systemThemes[id].displayName`.
- Each card renders:
  - Theme stripe (`bg-(--color-system-accent)` from per-system theme).
  - System name + game count (derived from `props.library.state.entries`).
  - 3 status rows (identified / covers / metadata) computed against
    the existing per-system MediaDb queries.
  - `[Freshen]` + `[Manage…]` buttons.
- Top-right of the section: single `[Freshen all systems]` button —
  iterates every system and triggers Freshen.
- Bottom: existing disk-usage summary stays.

**Card status computation:**
- `identified N/M` — count of library entries in this system with
  `sha1 != null` against total entries.
- `covers N/M` — count of library entries with a `box-front` cover in
  MediaDb against total entries.
- `metadata N/M` — count of library entries with a non-empty `year`
  OR `genre` OR `developer` OR `publisher` in MediaDb GameMetadata
  against total entries.

A status row is `✓` when N === M, `⚠` when 0 < N < M, `✗` when N === 0.

**Freshen behavior:**
- For each system, run the same pipeline `Sync media` does today,
  PLUS pre-step `Identify ROMs` if not 100% identified, PLUS
  `Sync metadata`. All three are already idempotent and already
  surface via Background Jobs.

**Exit criteria:**
- Per-system row list is gone; replaced by card grid.
- Cards in alphabetical order.
- Preferences sit at the top in a clearly-labeled card.
- Disk-usage line remains at the bottom.
- `[Freshen]` button on a card kicks off the right ops via Background
  Jobs (visible on the bar at the bottom).

### Phase 5 — Manage… side panel

**Scope:**
- New `GameMediaManagePanel.tsx` component — slide-in side panel
  from the right (z-50 or so, above page chrome but below modal
  layer).
- Props: `systemId` + `onClose` + the same handlers the cards used
  before consolidation (`startSync`, `startMetadataSync`,
  `startClearMetadata`, `startHashSync`, `startHashResolve`).
- Panel content:
  - Header — system name + last-synced timestamp + close button.
  - 5 op blocks, each with title + 1-line description + current
    count or status + `[Run]` button (disabled when this system is
    already busy on that op).
  - Footer — `[Done]` (just closes panel — ops continue in
    background).
- Esc closes; clicking outside closes; the bar at the bottom keeps
  showing in-flight jobs.

**Exit criteria:**
- Clicking `[Manage…]` opens the panel for that system.
- Switching cards while the panel is open re-targets the panel
  (doesn't require close + reopen).
- Each `[Run]` button kicks off the right op via the existing
  pipeline (no behavior changes to the underlying ops).

### Phase 6 — Tests + docs + cleanup

**Scope:**
- `cargo test --workspace -p oa-shell` passes (660+ tests).
- `npm run typecheck` silent.
- Update `docs/ACTIVE_WORK.md` with the in-flight stream entry while
  shipping; flip to "Recently completed" at merge time.
- SESSION_LOG entry under `docs/SESSION_LOG.md` summarizing the arc.
- Confirm with operator before merge.

---

## Out of scope (parking-lot candidates)

- **Animated card transitions** — cards fading/sliding when status
  changes. Visual polish; defer until the static version reads well.
- **Per-card progress overlay** — showing live `Sync media` progress
  inside the card itself (in addition to the BackgroundJobsBar). The
  bar is the canonical surface; cards just refresh status when ops
  finish.
- **Status sort / filter** — operator wants alphabetical, period. A
  "show only incomplete" filter is conceivable later but not now.
- **Cores tab content rework** — Cores tab gets `CoresCategorySettings`
  verbatim; deeper Cores UI work (curated tier picker, etc.) is a
  separate plan.
- **System Health card on HOME** — surfacing the same rollup as a
  card on the Retroverse HOME tab is plausible but separate.

---

## Risk list

- **Tab-state ownership.** The Overview rollup cards need to drive
  the parent SystemHealthPage's active-tab signal. Pass setter via
  props; trivial.
- **Cards layout on small viewports.** 3-col grid → 2-col fallback
  → 1-col fallback. Use CSS `auto-fit` minmax. The existing
  Retroverse grid patterns already do this.
- **Side-panel z-index vs BackgroundJobsBar.** Bar is at z-55. Panel
  needs to sit above page content (z-30-ish) but below modals (z-50).
  Pick z-45 or so; verify against the existing modal stack.
- **Freshen-all-systems blast radius.** Kicking off N×3 jobs at once
  needs to play nicely with the per-kind concurrency limit (one job
  per kind at a time). Should queue, not stack. The existing
  JobRegistry already enforces this.

---

## Reference

- Conversation that locked the design: 2026-06-03 session, three
  AskUserQuestion rounds.
- ASCII mockups: see the System Health and Game media full mockups
  shown to the operator before this doc was written. They map 1:1 to
  the phases above.
