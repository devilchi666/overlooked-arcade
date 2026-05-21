# neogeo Decisions Log

Append-only. Newest at the bottom.

---

## 2026-05-20 — FBNeo as the Neo Geo core

**Decision:** `default_core_dll_for_system("neogeo")` returns
`"fbneo_libretro.dll"`. No alternates registered in the per-system
Cores catalog because no practical alternate exists in the libretro
ecosystem.

**Why:** FBNeo (Final Burn Neo) is the canonical libretro Neo Geo
emulator. Multi-arcade core covering Neo Geo + CPS-1/2/3 + Toaplan +
Cave + etc., with Neo Geo as its most-validated subset. MAME proper
also drives Neo Geo but at much higher CPU cost and slower compatibility
cadence; standalone Neo Geo emulators (NeoRageX, MAME-NeoGeo, etc.)
aren't shipped as libretro cores.

**Considered and rejected:**
- **MAME as default.** Higher CPU cost, slower compatibility iteration.
  Operators who want MAME's accuracy on Neo Geo specifically can swap
  via per-system Cores override once the MAME .dll is in place.

---

## 2026-05-20 — AES + MVS share one SystemId

**Decision:** Neo Geo home (AES) + arcade (MVS) live under a single
`neogeo` SystemId. The AES/MVS-mode toggle (a runtime FBNeo core
option) is exposed via the per-system Core Options surface; no
sidebar split.

**Why:** AES and MVS are the same hardware running the same ROM-sets.
The differences (coin slot vs home pad, soft-dip switches, attract-mode
text) are cosmetic and per-game-configurable via the mode toggle.
Splitting them into separate slugs would force the operator to
duplicate library entries for every cross-format title (most of the
Neo Geo library) and choose AES vs MVS at scan time — which is
fundamentally a per-launch choice, not a per-library-entry choice.

**Considered and rejected:**
- **`neogeo-aes` + `neogeo-mvs` separate slugs.** Forces duplicate
  library entries + decision at scan time. Defeated by the AES/MVS
  difference being purely runtime-configurable.

---

## 2026-05-20 — Neo Geo CD is a separate SystemId (CD load path differs)

**Decision:** Neo Geo CD lives at the `neocd` slug, distinct from
cart `neogeo`. The two systems share the controller via
`"neogeo" | "neocd" => ...` dispatch arms in `bindings.rs` (same
precedent PCE-CD / TG-16 set and segacd / genesis set).

**Why:** Cart and CD use different cores (FBNeo for cart, NeoCD for
CD), different BIOS files (`neogeo.zip` vs `neocd_z.rom` /
`neocd_t.rom`), and different load paths (Bytes for `.neo` carts,
Path-based + BIOS pre-check for `.cue` / `.chd` CD images). Lumping
them under one slug would create operator confusion about which
BIOS / core / extension each library entry needs.

The 100+ Neo Geo CD-exclusive titles (Samurai Shodown RPG, Riding
Hero Special, etc.) also justify their own sidebar shelf.

---

## 2026-05-20 — `.zip` content-peek disambiguation (peek_zip_for_neogeo)

**Decision:** The library scanner runs a content-peek check on every
`.zip` file (`archive::peek_zip_for_neogeo` in `apps/oa-shell/src/archive.rs`).
Zips containing files matching `*.p1` AND `*.s1` (the Neo Geo
program-ROM + sprite-font signature) emit a `systemHint: "neogeo"`
that the frontend ingest path uses ahead of the generic extension
mapping. MAME zips fall through to the normal extension-based
classification.

**Why:** Both Neo Geo and MAME use `.zip` MAME-style ROM-sets, so
extension alone can't disambiguate. Operator was explicit about
preferring content-peek over per-folder Import Wizard rules for this
case (chose "Plan B: .neo + .zip with content-peek" over the
recommended `.neo`-only option during onboarding). The `.p1 + .s1`
signature is essentially unique to Neo Geo — Capcom CPS boards use
different file naming, MAME hardware blobs use per-board extensions
(`<game>.6e`, `<game>.5f` etc.), so the false-positive rate is
effectively zero.

Content-peek runs once per `.zip` during scan, ~1ms cost per file.
Operator with a 500-game Neo Geo + MAME mixed library pays ~500ms
total at scan time — acceptable.

**Considered and rejected:**
- **`.neo` only (Phase 0 recommended).** Simpler, but operators with
  MAME-style `.zip` Neo Geo libraries would have to convert or use
  per-folder Import Wizard rules. Operator overrode the default.
- **`.zip + .neo` naive (no peek).** Would force every `.zip` to
  classify as neogeo by default (since extension matching is first-
  wins), breaking MAME zip classification. Worse than the recommended
  approach.

---

## 2026-05-20 — Deepest+most-saturated red at hue 18° (cluster bottom)

**Decision:** `[data-system="neogeo"]` ships `oklch(0.50 0.27 18)` —
deepest lightness + highest chroma in the warm-red cluster (VB 7°
L=0.55 / MAME 12° L=0.64 / NES 28° L=0.62).

**Why:** Period-correct to SNK's 1990-2004 arcade marketing palette
(boxart, marquees, MVS cabinet sides, AES Big Box sleeves all used
cherry-red on black). The L+C profile makes neogeo the "cluster
bottom" — darker than every other red in OA's lineup, AND most
saturated. Reads as distinctly "SNK arcade cherry" rather than
scarlet (MAME), neon LED (VB), or crimson (NES).

Operator accepted the cluster crowding for period-correctness (same
acceptance pattern that locked saturn 275° in the violet cluster).

---

## 2026-05-20 — Phase 0 BIOS pre-check is existence-only

**Decision:** `check_neogeo_bios` checks only whether `neogeo.zip`
exists in `<exe_dir>/system/`. No SHA-1 verification of the zip
content. Returns `BiosCheck::OkCanonical` with a placeholder "EXISTS"
string for the sha1 field.

**Why:** Neo Geo's BIOS is a multi-ROM `.zip` whose content SHA-1
varies by:
1. MAME revision (each MAME release recomputes the zip contents +
   per-file metadata).
2. Universe BIOS (Unibios) presence — community BIOS hack adding
   region toggle + cheat menu that operators commonly install
   INSTEAD of the canonical MAME zip.
3. Operator-customized BIOS zip configurations.

A hardcoded SHA-1 list would flood operators with OkUnknownHash
warnings for legitimate BIOS variations. Existence-only is the
correct Phase 0 trade-off — catch the most common operator error
(no BIOS at all) without false-positiving on every BIOS variant.

FBNeo handles content validation internally if the file exists;
content errors surface as core load failures rather than OA pre-check
warnings.

**Phase 2 polish:** peek into the zip and verify canonical BIOS ROM
files (`sp-s2.sp1`, `sm1.sm1`, `lo-s.s2`, `sfix.sfix`, etc.) are
present — better than existence-only, less brittle than SHA-1.
