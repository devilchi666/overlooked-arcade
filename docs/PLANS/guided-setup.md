# Guided Setup — Plan

**Status:** Planning. No code. Locked design after the 2026-05-25 advisor + operator planning session.

**Author:** Operator + Claude (LLM pair).

**Owner-of-decisions:** the operator. This document records what was decided; revisit if any "locked" call below stops feeling right.

---

## 1. TL;DR

Upgrade the existing Import Wizard into a **guided-setup flow** that handles smart ROM/system matching, per-system readiness, curated core recommendations by CPU tier, optional canonical folder layout, and controller-navigable UI from day one. The same readiness component surfaces as a persistent Settings page. Existing operators get the upgrade on-demand. Two-tier model throughout: smart defaults for the 80% case, full customization escape hatch for the 20% who want it.

Positioning: "Guided Auto-Setup, not magic." Tell the operator what we did and why; never hide actions behind silent automation. Aligns with OA's curator audience.

---

## 2. Goals + non-goals

### Goals

- **Drop ROMs → it works.** A new user with a ROM collection should reach "I'm playing a game" without reading documentation.
- **Couch gamer experience.** Controller-navigable from the moment the wizard opens. Operator never needs to grab a mouse during onboarding.
- **Smart defaults, no surprises.** Every automation is visible — operator sees what was decided and why.
- **Existing operators benefit too.** Same wizard, accessible from Settings, can re-scan an existing library and surface missing pieces (cores, BIOS, per-game overrides).
- **Two-tier UX.** Sane defaults for most operators; power-user customization always one click away.
- **First-time friendly + defector friendly.** Both audiences walk the same flow; progressive disclosure means defectors aren't patronized and first-timers aren't lost.

### Non-goals

- **No magic detection.** We don't claim to know what every obscure file is. When we don't know, we surface it for operator decision.
- **No silent network calls.** Any download (cores, art-packs, BIOS lookup) requires operator confirmation.
- **No forced canonical layout.** Operators who like their existing folder structure keep it. The canonical layout is opt-in.
- **No full kiosk-shell controller polish.** Controller navigation in v1 covers the wizard + library browse/launch (per the agreed scope). The full kiosk Phase 1 (theme substrate, attract mode, perf budgets) remains separate.

---

## 3. Audience

**Primary: couch gamers.** Operator on a couch with a controller, monitor or TV across the room, wants OA to "just work" and stay out of the way. Will set up once and play often.

**Secondary: cabinet builders.** Eventually serviced by the full kiosk shell (Phase 1 of `docs/features/kiosk-shell/KIOSK_PLAN.md`). The guided-setup work pulls controller navigation forward so this audience isn't blocked entirely.

**Tertiary: desktop users.** The original OA audience — mouse + keyboard, curating per-system collections. Already served by the existing UI; guided setup doesn't degrade anything for them.

**Onboarding order: desktop → couch → cabinet.** Every user encounters OA first on a desktop install. From there they may move to couch (TV + controller) or cabinet (full kiosk). Guided setup is the first step that EVERY new user walks, regardless of where they end up.

---

## 4. Voice + tone

**Combo of warm/welcoming + curator/enthusiast.** Warm-but-not-saccharine, knowledgeable-but-not-condescending. The Jaguar gold theme and per-system care already imply this voice; the copy should match.

### Voice card — sample copy

| Situation | Bad | Good |
| --- | --- | --- |
| Scan complete, mixed library | "240 files scanned, 12 systems detected." | "Found 240 games across 12 systems. Quite a collection — let's get them ready." |
| Missing BIOS | "Error: scph1001.bin not found." | "PlayStation needs a BIOS file to run. We're looking for `scph1001.bin` — drop it in the BIOS folder when you've got it." |
| Default core picked | "Default core: snes9x_libretro.dll" | "Using snes9x for SNES — solid balance of accuracy and performance. You can swap to bsnes later if you want higher accuracy." |
| Unknown file | "Unknown file: weirdgame.romz" | "Not sure what `weirdgame.romz` is — looks like a ROM but the extension doesn't ring any bells. Set the system below, or skip it." |
| First-system bindings | "SNES default bindings applied." | "Defaults for SNES are set up — d-pad for movement, B for A, A for B (Nintendo convention). Looks good, or want to adjust?" |

The voice acknowledges the operator as someone who knows what they're doing without assuming they remember every system's quirks.

---

## 5. The wizard flow

### Step 0 — empty state (first launch only)

OA detects "no library configured" → main UI shows a friendly empty-state with a single `Set up your library` button. No auto-fire. Operator clicks → wizard opens.

Empty-state copy: "Drop in your ROMs and OA will get them ready. We'll detect systems, pick cores that match your hardware, and walk you through anything that needs your input. [Set up your library]"

### Step 1 — pick your ROM source folder

"Where are your games today?" Operator picks a folder. Could be a flat dump, a nested per-system layout, or anything in between — the smart scan figures it out.

### Step 2 — pick the canonical layout root (optional)

"OA can organize your library into a clean `<root>/<system>/` layout. You can keep your existing layout — OA will just read in place."

- **Default location** (mode-aware):
  - **Portable mode** (`portable.txt` present): `<exe_dir>\roms\<system>\`
  - **AppData mode**: `~\Documents\OverlookedArcade\roms\<system>\`
- Operator can pick a different location. Persisted to settings; future imports go to the same place.
- **Default state: skip.** Don't auto-move ROMs. The "organize my ROMs" toggle is OFF unless operator explicitly enables it. Moving 500GB of ROMs is heavy and risky (CD `.cue + .bin` siblings, archive inner-path encoding, watcher conflicts during move) — opt-in only.

### Step 3 — smart scan

Hash → header → extension → folder-hint detection pipeline. Hash matches are highest-confidence; folder-hint is a last resort.

Per-system findings emitted as a stream — operator sees the live count climb as scanning progresses. Per-archive logging already shipped (`scan_service: peek archive ...`) so a slow scan stays diagnosable.

**Scan-time hashing strategy is deferred to implementation** — likely a "quick scan (extension-based) vs deep scan (hash-verified)" toggle, or hash everything in the foreground for small libraries and stream in the background for large ones.

### Step 4 — per-ROM results table (LaunchBox-inspired)

Centerpiece screen. One row per ROM the scan found. Columns:

- **File** (the actual filename)
- **Detected system** (the auto-classification; clickable to override)
- **Suggested title** (cleaned: `Super Mario World (USA) [!].smc` → `Super Mario World`)
- **Confidence** (Hash / Header / Extension / Hint — tells the operator how strong the detection is)
- **Status** (✓ ready / ⚠ needs attention / ✗ blocked)

**Per-row actions (v1):**

- **Change system** — override the auto-detected classification (dropdown of known systems)
- **Edit title** — modify the cleaned title (inline text edit)
- **Skip** — don't import this ROM (toggle)
- **Show path** — popover with full filesystem path (helpful for "which copy is this?")

**Per-row actions deferred to v2** (documented for future addition; see §15):

- Mark as duplicate of X (when scan found two copies of the same game by hash)
- Pick a specific core for THIS ROM (per-game core override from this screen)
- Open in file explorer (jump to the file)
- Set as region representative (when multiple regions of the same game are imported, pick which is primary)

Sort/filter the table by any column. Bulk-select with checkboxes for bulk skip / bulk system change.

### Step 5 — per-system readiness checklist

After the per-ROM table, the wizard shows a single summary screen: one row per system found, with status:

- ✓ **Core installed**
- ✓ **BIOS present** (or ⚠ — needed but missing; ↪ — not required)
- ✓ **Default bindings ready**
- ✓ **Canonical folder set up** (only if operator opted in to canonical layout)
- ✓ **Core options pre-tuned** (per-system core options applied; ↪ if none)
- ⚠ **Per-game overrides from KNOWN_GAME_BUGS** (will be applied at commit; expandable to see which titles)

Each row expandable to show details. Issues are addressable inline (e.g., "Resolve" button next to ⚠ BIOS to open the BIOS folder + show the expected filename + SHA-1).

**Same component reused as a Settings page.** `Settings → Library → System Readiness` shows the same checklist for the current library state. Operator can revisit after dropping a new BIOS in, plugging a new controller, etc.

### Step 6 — resolve missing pieces

**Missing cores:** bulk-prompt. "We need 4 cores for these systems. Download from libretro buildbot? `[Download all (12 MB)]` `[Skip]` `[Pick individually]`". On-demand only — never silent.

**Missing BIOS:** guided UI. "PlayStation requires `scph1001.bin` (SHA-1 `xxx`). [Open BIOS folder] [Where to get it]". Never auto-download BIOS (legally sketchy).

**New systems encountered for the first time:** show the default bindings (see §6).

### Step 7 — first-system bindings setup

For every system that's NEW to this OA install (the operator has never imported a game of that system before), show a card:

```
SNES — controls ready
─────────────────────
D-pad      → SNES d-pad
B          → A (Nintendo convention)
A          → B
X          → Y
Y          → X
L / R      → L / R
Start      → Start
Select     → Select

Looks good?  [A: Looks good]   [Y: Customize]
```

Operator hits A to confirm; flow continues. Operator hits Y to drop into the existing Bindings UI for that system, customize, return.

For systems the operator has already configured before (e.g. re-entering the wizard via Settings to add games), no bindings card appears.

### Step 8 — confirm and commit

Final summary: "X games across Y systems. Ready to add to your library?"

Optional auto-actions surfaced as checkboxes (defaults indicated):

- ☐ Move/copy ROMs to canonical layout (default OFF — opt-in only)
- ☑ Sync cover art from libretro-thumbnails (default ON; existing behavior)
- ☑ Apply per-game core overrides from KNOWN_GAME_BUGS (default ON)
- ☑ Apply curated core selection based on detected CPU tier (default ON; CPU tier shown — see §7)

[Commit] writes to SQLite, registers folders for watcher, runs the auto-actions.

### Step 9 — done

Wizard closes. Operator lands in the library view with games visible.

**Post-commit destination is deferred to implementation** — options include going straight to library view (default), launching the first game LaunchBox-style, or a "what's next?" suggestions screen. Pick at implementation; lean library view for v1.

---

## 6. Folder model

**Two paths, both supported:**

- **Read-in-place** (default). OA indexes ROMs from wherever they live today. No file moves. Existing behavior, unchanged.
- **Canonical layout** (opt-in). OA proposes `<root>/<system>/` structure; operator can have OA copy or move ROMs there during commit. The `<root>` defaults to mode-aware location (see Step 2).

**Canonical layout is never forced.** Operators with pre-existing organized ROM collections (No-Intro, TOSEC, ROMVault-managed) keep their layout. The canonical-layout offer is for new users without a system, not a default.

**Moves are atomic per-folder.** A move that fails mid-way doesn't leave a half-moved system. Operator can cancel a long move; partial moves rollback.

**Heavy operations are backgrounded.** A 500GB ROM-copy doesn't lock the wizard. Progress shown; wizard usable for other actions.

---

## 7. Core selection — curated decision tree

### Decision locked 2026-06-06: heuristic is source of truth

The Phase 2 tier picks are driven by a static `sysinfo`-based heuristic — CPU brand + base clock + physical cores → High / Mid / Low bucket → per-system tier table → core recommendation. Real-time benchmarks were considered and explicitly deferred per the 2026-06-05 evening planning conversation.

**Rationale:** Most operators will never run a benchmark. The heuristic has to stand on its own for the common case (mid-range modern desktop, mid-range modern laptop, Steam Deck). Treating Phase 2A as a placeholder until benchmarks land would breed sloppy thresholds; treating the heuristic as the source of truth forces the tier thresholds to actually be defensible.

**Known limitations (accept, don't fix in Phase 2):**
- Steam Deck (Zen 2 mobile, weak CPU rating but strong iGPU) underrates against its actual emulation capability.
- Old high-end Xeon workstations rate Mid/High by core count but lose to modern Ryzen 5600G at every real emulation workload.
- Thin-and-light laptops with thermal throttling rate by boost clock, not sustained performance.
- GPU-bound cores (Beetle PSX HW, Flycast, PCSX2) aren't well-classified by CPU heuristic at all.

The operator override path (Settings → Performance → CPU Tier: Auto / High / Mid / Low) is the documented escape hatch for misclassified hardware.

**Possible Phase 2B (not committed):** Synthetic-stress benchmark mode triggered from a "Refine recommendations" button in Settings → Performance. Would run each multi-tier-eligible core for 10–30 s with a no-ROM boot OR a CC0 homebrew stress ROM per system, measure frame-time stability, override the heuristic per-host. Shader-pass benchmarking rides along. Architecturally additive — `recommended_core_for_system(system_id, benchmark_results)` falls back to the tier-from-heuristic when `benchmark_results = None`. Skipped for now; revisit only if operator feedback names the heuristic as the bottleneck.

### Tier detection

**CPU detection:** `sysinfo` crate. Read CPU brand, base clock, physical cores. Bucket into three power tiers:

- **High:** modern (Intel 10th gen+ / AMD Ryzen 3000+) with ≥6 cores ≥3.0 GHz base, dedicated GPU
- **Mid:** Intel 7th-9th gen / AMD Ryzen 1000-2000, 4+ cores ≥3.0 GHz, integrated or modest GPU
- **Low:** older / mobile CPUs, <4 cores, <3.0 GHz, integrated GPU only

The tier is computed once at first launch + cached. Operator can override in Settings → Performance → CPU Tier (drop-down: Auto / High / Mid / Low).

**Per-system core preferences:** declarative table, one entry per system that has multiple core options:

```
psx:
  high → beetle_psx_hw_libretro      (Vulkan, PGXP enabled by default)
  mid  → duckstation_libretro        (best mid-tier balance)
  low  → pcsx_rearmed_libretro       (lightest, fewer features)
  notes: "DuckStation is the default for unknown tier."

snes:
  high → bsnes_libretro              (highest accuracy)
  mid  → snes9x_libretro             (default; solid balance)
  low  → snes9x_libretro             (still fine on low tier)

n64:
  high → mupen64plus_next_libretro   (LLE GPU plugin, modern)
  mid  → mupen64plus_libretro        (HLE, faster)
  low  → parallel_n64_libretro       (lightweight)

# Systems with no tier-based variation (e.g. tg16, gba) use their
# default_core_dll_for_system entry directly.
```

**Surfaced to operator on the readiness checklist.** Each system row shows "Picked: beetle_psx_hw_libretro (high-tier core)" — visible automation, not silent.

**Override path:** per-system Settings → Cores. Per-game Settings drawer also has a core override. Both already exist; wizard just feeds reasonable defaults into them.

---

## 8. Per-game overrides from KNOWN_GAME_BUGS

The wizard reads `docs/cores/<id>/KNOWN_GAME_BUGS.md` files at commit time. Entries that name a specific core / option / setting for a known-buggy game pre-fill the per-game `GameOverrides` for matching imports.

Example KNOWN_GAME_BUGS entry:

```
## Goldeneye 007 — runs poorly on parallel; use mupen64plus_next
- system: n64
- match: hash:abc123 OR title:"GoldenEye 007"
- override: { libretro_core: "mupen64plus_next_libretro.dll" }
- reason: "ParaLLEl GPU plugin has timing bugs on this title; mupen64plus_next renders correctly."
```

Wizard sees the import matches, pre-fills `GameOverrides.libretro_core` for that game. Shown to operator on the readiness checklist row ("3 per-game overrides will be applied"); expandable to see which.

**File format for KNOWN_GAME_BUGS is deferred to implementation.** Today these files are free-form markdown; for auto-application we may need a structured front-matter or sidecar YAML. Don't block planning on this.

---

## 9. Help + tip suppression

**Per-dialog "Don't show this again" checkbox.** Every tip / help dialog / "did you know" prompt has a checkbox at the bottom: "Don't show this again."

**Criticality tier — load-bearing alerts can't be suppressed:**

- **Tip / help** — fully suppressible. Examples: "Hover a game tile to see details", "Click here to organize by system", first-launch onboarding tips.
- **Soft warning** — suppressible per-occurrence, not permanently. Examples: "Cover art sync took longer than expected", "Watcher detected new files but auto-scan is off".
- **Load-bearing** — never suppressible. Examples: "Jaguar requires jagboot.rom — game will not start", "Save state slot full", "Core failed to load".

**Re-enable path: Settings → Help → Tips & Notifications.** List every suppressed tip with a per-item "Show again" toggle, plus a master "Reset all suppressed tips" button. Load-bearing alerts don't appear in this list (they were never suppressible).

**Master toggle: Settings → Help → "Expert mode".** Single switch that suppresses ALL tier-1 tips at once. For defectors who want zero hand-holding from day one. Doesn't touch load-bearing alerts.

---

## 10. Controller navigation

**Model: DPad + focus rings (Steam Big Picture style).**

- D-pad / left stick → move focus between UI elements
- A → confirm / select / advance
- B → cancel / back
- X → secondary action (per-screen contextual; surfaced in on-screen hint bar)
- Y → tertiary / customize / "tell me more"
- Start → menu (when applicable)
- Select → help overlay (shows the controller hint for current screen)
- Shoulders → move between tabs / sections within a screen

**Scope: wizard + library browse/launch only for v1.** Per the agreed scope, controller navigation covers the onboarding wizard AND library browsing AND launching a game. Per-game settings drawer, cheat editor, complex configuration screens stay mouse + keyboard until kiosk shell Phase 1 ships properly.

**Visual:** subtle focus ring (high-contrast outline on focused element). Animation between focus moves is short (≤120ms). Operator always knows what's selected.

**On-screen hint bar:** persistent footer showing button hints for current screen. "A: Confirm    B: Back    Y: Customize    Select: Help". Hints change per screen.

**Mouse + keyboard still work alongside controller.** Operator can mix input modes freely. Click anywhere with mouse; focus follows.

**Implementation primitives needed:**

- Focus manager (which element is currently focused; how DPad moves between elements)
- Controller-to-UI input layer (gamepad button events → UI events without conflicting with the emulator core)
- Focus-ring CSS / component pattern for every focusable element
- Hint-bar component

This is meaningful new work. Estimated ~2-3 weeks of frontend infrastructure before any wizard step gets controller-navigable. Could be a Phase 0 of the implementation (build the controller-nav primitives), then Phases 1+ wire each wizard step to use them.

---

## 11. Behavior for existing operators

**Wizard accessible from Settings.** `Settings → Library → Re-scan with smart detection` (new) opens the same guided wizard, pre-pointed at an existing library folder. Existing operators can opt in to the upgrade — re-detect system classifications, apply curated core selection, apply per-game overrides from KNOWN_GAME_BUGS, get the readiness checklist for systems added since their original setup.

**Existing setup is preserved by default.** Re-running the wizard never destroys an existing override or binding. Detected improvements are SUGGESTED; operator confirms each.

**No proactive prompts.** Don't show existing operators a "we have a new feature, want to re-scan?" toast. The Settings entry is the discovery path; operator opts in when they want to.

---

## 12. Information architecture

**Where things live in Settings after this lands:**

```
Settings →
  Library →
    Folders (existing)
    Watcher (existing)
    Re-scan with smart detection ← NEW: opens the wizard
    System readiness ← NEW: same component as wizard Step 5
  Performance →
    CPU tier ← NEW: Auto / High / Mid / Low override
    Recommended cores ← NEW: view the per-system tier picks; override any
  Help →
    Tips & notifications ← NEW: suppressed-tips registry + master "Expert mode"
```

**Wizard entry points:**

- First launch empty state → big "Set up your library" button
- Settings → Library → Re-scan with smart detection
- Watcher detects an unrecognized folder (DEFERRED — see §15) → toast "New folder detected, run smart scan?"

---

## 13. Implementation phases

Suggested split into phases that ship cleanly:

### Phase 0 — Controller-nav primitives (~2-3 weeks frontend)

Build the infrastructure that everything else depends on:

- Focus manager
- Gamepad input → UI event layer
- Focus-ring component pattern
- On-screen hint-bar component
- Settings page for controller-nav (turn off entirely; remap nav buttons)

**Shippable independently.** This work makes the EXISTING UI controller-navigable for a few screens (library tile grid, settings) as a proof-of-concept. Operators get a small win immediately.

### Phase 1 — Wizard upgrade (~3-4 weeks)

- Smart scan with hash → header → extension → folder-hint pipeline
- Per-ROM results table (v1 actions only)
- Per-system readiness checklist component
- Wire bulk-prompt for missing cores (uses existing `core_installer.rs`)
- Wire guided BIOS resolution UI
- Voice/tone copy pass on every existing wizard string

### Phase 2 — Curated core selection (~1 week)

- `sysinfo` integration for CPU tier detection
- Per-system tier preference table (the §7 table)
- Apply at commit (per-system + per-game core overrides)
- Settings → Performance → CPU tier override
- Settings → Performance → Recommended cores (visible per-system picks)

### Phase 3 — Folder management (~1 week)

- Canonical layout proposal in Step 2
- Atomic per-folder move/copy with progress + cancel
- Watcher conflict handling during moves
- Mode-aware default root

### Phase 4 — First-system bindings + KNOWN_GAME_BUGS overrides (~1 week)

- "Looks good?" bindings card per new system encountered
- KNOWN_GAME_BUGS sidecar/front-matter format design
- Auto-application of per-game core overrides at commit
- Surfaced on the readiness checklist before commit

### Phase 5 — Help suppression registry (~3-4 days)

- Per-dialog suppression key
- Settings → Help → Tips & notifications panel
- Master "Expert mode" toggle
- Criticality tier enforcement (load-bearing alerts can't be suppressed)

### Phase 6 — Existing-operator re-entry (~3-4 days)

- Settings → Library → Re-scan with smart detection entry point
- Override-preservation logic (never destroy existing bindings/overrides)
- "Detected improvements" diff view before commit

### Total

Roughly 8-10 weeks of focused work, depending on how parallelizable. Phase 0 is the longest single chunk; Phases 1-6 can pipeline if multiple sessions overlap.

---

## 14. Open questions deferred to implementation

These don't need answers to plan; they need answers during the build.

1. **Scan-time hashing strategy.** Foreground (slow but accurate) / background streaming (faster perceived) / on-demand (lazy, only for ambiguous classifications)? Likely a "quick scan" vs "deep scan" toggle in Step 3.
2. **Post-commit destination.** Library view (default), play-first-game LaunchBox-style, or "what's next" suggestions screen? Lean library view for v1.
3. **Wizard cancellation behavior on first launch.** What if the operator closes the wizard mid-flow? Show empty library? Show a "come back to setup" reminder? Save partial progress?
4. **DPad-only vs DPad + sticks for nav.** Sticks doubling as focus navigation is conventional but can conflict with analog stick input the operator's controller sends idle. Default DPad-only; sticks as opt-in?
5. **Watcher-triggered "we found something new" toast.** When the folder watcher detects a new file in a registered folder, do we proactively offer "smart scan this?" Or silently add it via existing scan path?
6. **KNOWN_GAME_BUGS sidecar format.** Free-form markdown today; structured front-matter / sidecar YAML for auto-application?
7. **Visual style of the focus ring.** Subtle outline vs glow vs bordered card vs accent-colored — needs design pass.
8. **Animation budget for nav transitions.** Snappy (no animation) vs subtle (≤120ms) vs animated-but-tasteful (200-300ms). Tradeoff between feel and perceived latency.
9. **First-launch empty-state visual.** Just a centered button, or something more visually engaging (hero image, sample-library tease)?
10. **Help-suppression registry storage.** Where does the suppressed-tips list persist — `appData/prefs.json`, a SQLite table, somewhere else?

---

## 15. v2 / future additions

**Documented now so they're not forgotten; not in scope for the initial guided-setup build.**

### Per-ROM table actions (v2)

- **Mark as duplicate of X** — when scan finds two copies of the same game by hash, link them; UI to pick which is primary.
- **Pick a specific core for THIS ROM** — per-game core override directly from the results table (alternative to going through Settings drawer).
- **Open in file explorer** — jump to the file location.
- **Set as region representative** — when multiple regions of the same game are imported, pick which is the primary tile.
- **Bulk edit operations** — apply a system/title/core to many selected rows at once.

### Wizard polish

- **Watcher-triggered smart scan toast** — proactive offer when new folder content appears.
- **Sample library tease** — first-launch empty state shows a few example tiles to convey "this is what your library can look like."
- **"Walk me through it" mode** — toggle for first-time emulation users; adds extra explanatory copy at every step.
- **Library export to LaunchBox-XML** — for dual-use operators syncing OA library to LaunchBox for the metadata OA doesn't have natively.

### Adjacent features mentioned during planning, not in scope

- **System Mode** (immersive per-system experience — boot animation, ambient music, UI transforms, navigation changes). High-effort polish; deferred to Phase 7+ kiosk shell or a dedicated arc.
- **Game Context System** (rich hover info, "if you like X try this", fun facts, dev/year, known issues). Could be a follow-up arc after guided setup ships. ChatGPT pitched this as 2-3 weeks of work.
- **Play History Intelligence** (track plays, surface "you seem to like SNES RPGs", hidden gems, dormant favorites). Original-to-the-space feature; logical follow-up to the curator audience. Multi-week effort.
- **RetroAchievements integration** — close one of two big RetroArch gaps; community demand is high; ~3-4 weeks. Pending strategic decision (open question from advisor brief).
- **Netplay** — close the other big RetroArch gap; multi-month, risk of shipping a worse version for years. Pending strategic decision.

### Strategic decisions accepted from advisor session (2026-05-25)

- **Theme ecosystem — wait.** Don't build now. Dead-ecosystem trap (no users → no themes → no users). Reconsider if/when the kiosk shell launches and there's a clear community pull.
- **License — go permissive eventually.** MIT or Apache 2.0 once the dynamic-load pivot fully lands (installer ships only our own DLL builds; GPL cores stay GPL in their .dll). Mission-aligned: encourages contributions + forks + ecosystem; commercialization risk is low (commercial actors will copy regardless; our advantage is vision + execution speed).

---

## 16. Related plans + dependencies

- **`docs/features/kiosk-shell/KIOSK_PLAN.md`** — the full cabinet experience. Guided setup pulls Phase 1's controller-nav primitives forward. The remaining kiosk Phase 1 work (attract mode, perf budgets, launch ceremony, controller binding wizard) stays in scope of that plan.
- **`docs/cores/<id>/KNOWN_GAME_BUGS.md`** — per-system bug files. The auto-application work (Phase 4) requires a structured front-matter or sidecar format; see §8 + §14.
- **`docs/cores/<id>/README.md`** — per-system bring-up docs. Should mention curated core tier picks once they exist.
- **`apps/oa-shell/src/core_installer.rs`** — existing core download infrastructure. Reused for the bulk-prompt missing-core flow (§5 Step 6).
- **`apps/oa-shell/src/rom_hashes.rs`** + **`apps/oa-shell/src/rom_header.rs`** — existing hash + header detection. Foundation for the smart-scan pipeline (§5 Step 3).
- **`apps/oa-shell/src/system_settings.rs`** + **`apps/oa-shell/src/library_db.rs` `GameOverrides`** — existing per-system + per-game settings. Wizard writes into both during commit.
- **`apps/oa-shell/src/light_gun_systems.rs`** — declarative system catalogue pattern. Curated core tier table (§7) follows the same shape.

---

## 17. What "ready to start" looks like

Before any code is written for this plan, the operator should:

- Confirm Phase 0 (controller-nav primitives) is the right first deliverable to ship independently.
- Decide whether to merge `feat/jaguar-keypad-passthrough` first (it's pushed but awaiting playtest with canonical jagboot.rom) so main is fully clean.
- Decide whether to update `docs/NEXT.md` to elevate guided-setup to HIGH priority (above current MEDIUM band items: vector-phosphor shader, vb-monochrome shader).
- Decide whether to write a one-time announcement / Q&A doc for the existing operator community about the upcoming change in onboarding flow.

None of those block writing this plan; they're just the natural next steps after planning closes.
