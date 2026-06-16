# External Emulator Depth — arc plan

**Status:** Planned 2026-06-15. No code yet. Slice 1 queued in
[NEXT.md](../NEXT.md) HIGH band.

**Owner-of-decisions:** the operator. This document records what was
decided in the 2026-06-15 planning session + the slice roadmap.

**Parent context:** Builds directly on the shipped launcher abstraction
(VL Phase C — `Launcher` trait + `ExternalProcessLauncher` +
`config/emulators/<id>.yaml` profiles) and the sketched VL **Phase D**
(install pipeline). See
[virtual-library-and-launcher-arc.md](virtual-library-and-launcher-arc.md)
§6 Phase D/F and the archived
[launcher-abstraction.md](../_archive/PLANS/launcher-abstraction.md).
Decisions: [features/external-emulators/DECISIONS.md](../features/external-emulators/DECISIONS.md).

---

## Goal in one line

Take OA's external-emulator support from "can launch a game" to a
managed, deepening relationship — OA can **install** the emulator,
**update its launch knowledge without an OA release**, and
progressively **control** it — building toward the long-term north star
of running the emulator *inside OA's own window*.

## Plain-English summary (the north star)

OA already plays most games itself. Some games only run in separate
programs (Dolphin, Cemu, RPCS3, …), and OA can already open those. This
arc goes deeper:

1. **Recipe upgrade** — make the little per-emulator "recipe" files
   handle the trickier emulators, and make those recipes **updatable on
   their own** so a changed emulator flag never forces a whole-OA update.
2. **"Install it for me" button** — OA downloads + installs the emulator,
   but **only ones it's legally allowed to** (green/yellow gate); never
   games, BIOS, or keys.
3. **Better control** — push settings in, per-game config, grab
   screenshots — built as updatable recipes, *leading toward* the
   eventual dream of wrapping the emulator into OA's own window (the
   thing almost every frontend does badly; we earn it, one emulator at a
   time).

---

## Load-bearing principle — recipes are updatable DATA, not baked-in code

The operator's central requirement (2026-06-15): **emulators change
their CLI flags and options constantly; that must NOT force a rebuild +
reship of all of OA.**

So the whole arc is shaped around this:

- All per-emulator knowledge lives in **data** — the
  `config/emulators/<id>.yaml` "recipe" files OA reads at startup (they
  already do; they're plain files at `<exe_dir>/config/emulators/`).
- Those recipes are **refreshable independently of the OA binary**,
  through the operator-initiated update channel designed in
  [content-packs.md](content-packs.md) (OA-curated registry as a JSON
  file in a public GitHub repo; manual "check for updates"; sha256
  verification). A changed emulator flag = publish a new recipe = user
  clicks "update recipes" = done. **No OA rebuild, no reinstall.**
- Compiled Rust stays a **thin, generic engine** that interprets
  declarative recipe data. The design target: *declarative-first with a
  code escape hatch* (mirrors the locked theming "low floor / high
  ceiling" philosophy). New emulator or changed flags → just data. Only
  a genuinely-new *mechanism* OA has never done before needs a code
  change + release. Be honest about that boundary — it's the line
  between "recipe update" and "OA update," and we keep the common case
  firmly on the data side.

---

## Locked decisions (2026-06-15)

Full text in
[features/external-emulators/DECISIONS.md](../features/external-emulators/DECISIONS.md).
Summary:

- **ED1 — Extended control = OA-authored adapters, NOT a third-party
  plugin SDK.** Reaffirms the 2026-06-02 plugin-API rejection (only the
  narrow operator-profile case was ever un-parked, 2026-06-03). OA owns
  the control logic in-tree; no public plugin contract for strangers.
- **ED2 — Per-emulator knowledge is updatable data, decoupled from the
  OA binary** (the load-bearing principle above).
- **ED3 — Install pipeline has a per-emulator legal gate.** Green = OA
  may download + install; Yellow = OA links to the official download
  only, user installs manually. **Default Yellow when unverified.** Zero
  ROMs / BIOS / keys, ever.
- **ED4 — Schema accretion, not one-profile-per-pair.** ares + BizHawk
  **auto-detect the system from the file** (verified 2026-06-15), so the
  per-system `--system` problem mostly dissolves — both get a single
  positional recipe. The real additions are a per-OS `binary_name` map
  and MAME's non-path content model. The optional `--system` fallback is
  a *reserved seam*, not built up front.
- **ED5 — Window-wrapping / embedding is the north star, deferred to its
  own later arc.** Architecture must not preclude it; not near-term;
  proven on one emulator first.
- **ED6 — "Control" capabilities are a separate axis from the Phase-C
  D5 `LauncherCapabilities`.** D5 = which OA QuickSettings an external
  exposes (all false today). The new control surface = OA driving the
  *emulator's own* config/state. Separate namespace; do not overload D5.

---

## Phase 1 — Recipe format upgrade + independent updates (foundation)

The smallest, lowest-risk thread. Closes the research doc's open schema
questions and delivers the operator's #1 ask (independent recipe
updates).

### Slice 1 — schema accretion + the three deferred profiles `[QUEUED]`

The queued first slice. Scope:

- **Per-OS `binary_name` map** — additive schema field
  (`binary_name` accepts either the current single string OR a
  `{ windows, macos, linux }` map). Loader picks the current OS; single
  string stays valid (backward-compatible). Grounded in the verified
  per-OS binary table in the research doc.
- **Author `ares.yaml` + `bizhawk.yaml`** — single positional
  `{content}` template (auto-detect confirmed; no `--system` needed for
  the common case). Maps each to the OA system ids it covers that
  already exist.
- **MAME (`mame` system) decision** — its content model is a short
  rom-set name + a configured `rompath`, not a file path. Slice-1 call:
  either add a small `content_mode` enum (`path` | `rom_name`) now, or
  defer the standalone-MAME profile (the in-process MAME core already
  covers it). Decide at execution start; lean toward `content_mode` only
  if it's a clean ~1-field add.
- **Reserve (document, don't build) the optional per-system `--system`
  fallback seam** for the rare ambiguous-extension case.
- Extend the existing `all_shipped_profiles_parse_and_hold_invariants`
  test to cover the new shape; keep the loader's skip-on-malformed
  resilience.

**Demoable acceptance:** ares/BizHawk launch a game from a normal OA
tile via the operator's own install; a profile authored with a per-OS
`binary_name` map resolves correctly on Windows.

### Slice 2 — recipe update delivery (the "no whole-OA update" win)

- A "check for emulator recipe updates" action that pulls refreshed
  `config/emulators/*.yaml` from the OA-curated registry, reusing the
  [content-packs.md](content-packs.md) channel (manual, sha256-verified,
  privacy-panel-disclosed). **Depends on the content-pack distribution
  infra, which is currently design-only** — this slice either rides that
  build or stands up a minimal profiles-only version of it.
- Operator-facing: Settings → External Emulators gains an "Update
  recipes" affordance + last-checked status.

---

## Phase 2 — Install pipeline ("install this emulator for me")

Builds on VL Phase D's `InstallableProfile` sketch. Decoupled into:
*install mechanics* (the plumbing) vs *new-system wiring* (onboarding a
console OA doesn't know yet).

### Slice 3 — legal gate + install mechanics on one easy case

- **Per-emulator legal classification** (ED3): a recipe field marking
  the emulator **Green** (OA may download + install) or **Yellow** (link
  to official download only). Default Yellow. UI: install cards show the
  right affordance per classification; DuckStation-class
  (CC BY-NC-ND / "no repackaging") emulators are Yellow.
- **Install mechanics** proven on **one clearly-Green, already-wired
  emulator** (e.g. an open-source one whose OA system id already exists —
  decouples install plumbing from new-system onboarding): fetch latest
  official release → download → extract → locate the binary → write the
  `emulators.json` path automatically. Background-Jobs-tracked. Default
  install location `<exe_dir>/Emulators/<id>/` (matches the existing MAME
  pattern).
- Explicit "OA does not provide ROMs or BIOS files" language on every
  install card.

### Slice 4 — version pinning + update cadence

- Per-emulator current-version status + "Update" + "Open install
  folder". Operator can pin a version (auto-update never breaks their
  setup). Configurable cadence (default: manual / weekly check).

### Slice 5+ — new-system wiring (rides the descriptor machinery)

**Explicitly decoupled** from install mechanics. Each section-B system
(Cemu→`wiiu`, RPCS3→`ps3`, plus Switch/3DS/Vita/Xbox/PS4/Model 3, and
Dolphin's Wii half) needs an OA system id + `config/systems/<id>/`
descriptor + sidebar/metadata before its (often already-CLI-verified)
profile is useful. Built on the per-system-descriptor loader. PS3 also
needs **directory-based content resolution** (EBOOT.BIN inside a game
folder, not one ROM file) + a **firmware precondition** (PS3UPDAT.PUP —
detect-and-prompt, never fetched). These ride one-system-at-a-time as
priorities allow; they are not gated on Phase-1/Slice-3 work beyond the
recipe + install foundations.

---

## Phase 3 — Extended control (adapters), building toward window-wrapping

OA-authored, recipe-driven control above `ExternalProcessLauncher`
(ED1/ED6). Grounded in what standalones actually expose — mostly
**pre-launch config-file injection + per-game config + post-hoc artifact
reading**, *not* live command channels (those barely exist outside
RetroArch). Declarative-first; code escape hatch only for the weird ones.

### Slice 6 — control-capability namespace + config-file injection

- A control-capability surface separate from D5 (ED6). First real lever:
  **declarative config-file injection** — a recipe describes "to set X,
  write key=value into config file Z" and the engine writes it before
  spawn. Pilot on one emulator with a clean config file (e.g. Flycast
  `emu.cfg`, Dolphin `.ini`, or RPCS3 `config.yml`).

### Slice 7 — per-game config

- Built on Slice 6: a per-title config dir / overlay where the emulator
  supports it (Dolphin GameINI, RPCS3 per-title).

### Slice 8 — artifact reading

- Read what the emulator leaves behind (screenshots, saves) from its
  known output dir — OA can surface/ingest, not command a capture.

### Slice 9+ — precondition hooks (the "plugin/script hook" case)

- Detect-and-prompt for hard preconditions: RPCS3 firmware, Cemu
  `keys.txt`, xemu MCPX. Never ship or fetch them — surface a clear
  readiness gate (like the BIOS gate).

### North-star (deferred to its own arc) — window-wrapping / embedding

Running the emulator's output *inside OA's window* so it feels like one
app. **Hard and fragile** (the OS fights you; breaks differently per
emulator — which is why frontends do it badly). Not near-term. The
recipe + control foundation above is what makes a careful, one-emulator-
first attempt *possible* later instead of a doomed bolt-on. Scoped as its
own focused arc when reached; architecture here must not preclude it
(ED5).

---

## Legal posture (hard rules — trust is the constraint)

- **Zero ROMs, zero BIOS, zero keys, ever.** OA points at what the user
  already has; downloads none.
- **Install only what we're legally allowed to** (ED3). Green = OA may
  download + install; Yellow = official-link-only. Default Yellow when a
  license is unverified — never auto-fetch on a guess.
- Firmware / keys (PS3UPDAT.PUP, `keys.txt`, MCPX) are **user-installed
  preconditions** OA detects and prompts for, never provides.

---

## Explicitly deferred / out of scope

- **A generic third-party plugin SDK** — stays rejected (ED1; 2026-06-02
  PARKING_LOT). This arc is OA-authored adapters only.
- **Window-wrapping / embedding** — north star, own later arc (ED5).
- **New-system onboarding ahead of the foundation** — wiring Wii U / PS3
  / Switch / etc. rides Phase 2 Slice 5+ on the descriptor machinery, one
  at a time.
- **Emulator binaries through the content-pack art/metadata channel** —
  no. Binaries are Phase-2 installer territory; the content-pack channel
  carries recipes + art/metadata/themes, not emulator executables.

## Dependencies

- **Phase 1 Slice 2** (recipe updates) leans on the
  [content-packs.md](content-packs.md) distribution infra (design-only
  today).
- **Phase 2 Slice 5+** (new systems) leans on the per-system-descriptor
  loader ([per-system-descriptors.md](per-system-descriptors.md)).
- VL **Phase D**'s `InstallableProfile` shape is the starting sketch for
  Phase 2.

## Verification approach

- Each slice: `cargo test -p oa-shell` green + frontend typecheck/lint +
  operator smoke playtest of the visible surface before merge.
- Slice 1: the strict all-profiles-parse test gates authoring; ares +
  BizHawk launch from a real tile.
- Slice 3: integration-test the actual download + extraction for the one
  Green pilot emulator (per the VL Phase D verification note).
- One branch per arc/phase per the operator's branch workflow; merge to
  main at playtestable milestones.

## Open questions deferred to execution time

- **MAME content model** (Slice 1) — `content_mode` enum now vs defer the
  standalone-MAME profile.
- **Slice 2 update infra** — ride a full content-pack build vs stand up a
  minimal profiles-only updater first.
- **Slice 3 pilot emulator** — which Green, already-wired emulator proves
  the install plumbing.
- **Slice 6 pilot emulator** — which clean config file proves injection.
