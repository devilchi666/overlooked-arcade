# Metadata Curation — Decisions

Append-only. Newest at the bottom. The *why* matters more than the *what*.

---

## 2026-06-11 — D1–D5 locked at planning (operator Q&A)

- **D1 — Location: its own `metadata` Settings category** (engine territory,
  CONTENT group), not on the library tile, not a per-game modal.
  *Why:* operator chose "its own tab in settings"; a registered category is
  layout-agnostic so it survives the upcoming settings-layout rework.
  Inline-in-library editing deferred to Wave 3 ("we may add inline later").

- **D2 — Override layer everywhere.** Edits stored as a sparse per-field
  override applied at read time (`override → enriched/baked → source`);
  reset = drop the row. System reuses the shipped `system_info_overrides`;
  game-factual gets a new `game_metadata_overrides` mirroring
  `game_info_overrides`.
  *Why:* operator picked the override layer; it gives free per-field reset +
  provenance + a natural undo unit + a clean diffable unit for the future
  "submit correction" flow, and never lets a re-sync clobber an edit or an
  edit destroy source. Matches the pattern already shipped twice (game-info,
  system-info).

- **D3 — Game edits key on `identity_id`,** not per-ROM.
  *Why:* tiles + detail render from the identity; that's the canonical edit
  target. Per-variant editing is a later Preservation-mode tier (composes with
  virtual-library Phase B).

- **D4 — The Settings tab owns its own searchable game/system list (picker).**
  *Why:* editing no longer lives on the tile, so the surface needs its own way
  to choose what to edit — OA's "library spreadsheet," housed in Settings, and
  the host for Wave-2 multi-select bulk edit.

- **D5 — Premium UX is a gated requirement, not deferred polish.** Operator:
  make it "a top-notch editor window with good features," explicitly "don't
  look like a Windows-98 tab."
  *Why:* metadata editing is OA's biggest interaction win over LaunchBox
  (research §4) — a flat property grid would squander it. Wave-1 exit requires
  operator sign-off on *feel* (live preview, typed controls, provenance dots,
  controller-navigable), not just function. See plan §"UX pillar".

## 2026-06-12 — S2 layout review (operator playtest of the first cut)

The first S2 cut rendered the editor *inside* the Settings 3-pane (a
sidebar-within-a-sidebar) and showed every field's provenance chip +
reset always-on across three columns — the operator's verdict was "very
very busy." Locked decisions from the review:

- **D6 — Metadata is a full-screen takeover, not a Settings sub-pane.**
  Entering the `metadata` category takes over the whole engine surface
  with a `‹ Settings` back button (returns to the category you came
  from). *Why:* the editor is itself a 3-zone surface (system list /
  form / preview); nesting it inside Settings' own category sidebar
  guaranteed cramping. Reclaiming the full width is half the fix.

- **D7 — Provenance collapses to a single "Default", revealed on
  hover/focus.** No always-on per-field chips. A field is a clean
  label+value at rest; an edited field carries a thin accent bar; the
  "Default: <value> · Reset" affordance appears only on row hover /
  focus-within (keyboard-reachable). The L1-vs-L2 distinction (MAME
  baseline vs curated) drops from the visible label into a hover tooltip
  only. *Why:* "MAME baseline / curated" is jargon most editors don't
  need; the always-on chips were the dominant source of visual noise.
  Matches the plan's actual pillar wording ("a *subtle* overridden dot
  … hover/expand shows the source value + reset").

- **D8 — Field groups are data-driven + disclosed via expanders.** The
  lead group ("Identity & hero") is open; deeper groups (Technical,
  Peripherals) start collapsed, with an edited-count badge so hidden
  edits are still visible. *Why:* 25 fields flat is a wall; and the
  operator flagged that which fields count as "hero" will change over
  time — so the grouping + open-by-default is a config array, never
  hardcoded markup.

- **D9 — Live preview is a collapsible docked panel (right).** Toggled
  from the top bar; off-state hands its width back to the form. *Why:*
  the preview is a real strength but shouldn't tax the heads-down
  editing case. Operator picked Concept A ("three-zone, docked
  preview") over banner / sub-nav / single-column variants.
