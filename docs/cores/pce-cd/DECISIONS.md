# pce-cd Decisions Log

Append-only. Newest at the bottom. Every entry: **what** we decided, **when**,
**why**, and **what we considered and rejected**.

pce-cd-specific integration choices live here. Project-wide decisions (engine
stack, license, libretro pivot, etc.) live in `docs/DECISIONS.md`. PC Engine
cart-specific decisions live in `docs/cores/tg16/DECISIONS.md`.

---

## 2026-05-18 — Beetle PCE Fast is the PCE-CD core

**Decision:** Ship Beetle PCE Fast (`mednafen_pce_fast_libretro.dll`) as the
default core for `pce-cd`. Full Beetle PCE Mednafen
(`mednafen_pce_libretro.dll`) stays available as a per-game fallback via the
per-system or per-game core override UI; users drop it in `<exe_dir>/cores/`
if a title needs it.

**Why:** Phase 0 Spike 2 noted that Beetle PCE Fast ships `pcecd.cpp` and
`pcecd_drive.cpp` — Fast already carries the CD path. 2026-05-18 operator
validation confirmed it: Castlevania: Rondo of Blood (CHD) boots, CDDA + FMV
+ gameplay all run on Fast against `syscard3.pce`. No need to vendor or load
the heavier full-Mednafen build by default. The HuCard core path (also Fast)
is unchanged, so cart games and CD games share the same .dll on disk; the
shell's libretro singleton constraint means only one instance is loaded at
a time anyway.

**Considered and rejected:**

- **Full Beetle PCE Mednafen as the default.** Heavier, slower load, larger
  memory footprint, no observable compatibility win for Rondo. Reserve for
  per-game escape hatch if a future title regresses on Fast.
- **Spike full Mednafen separately before committing.** Bypassed because
  Fast already passed validation; spiking what we don't need is premature.

---

## 2026-05-18 — pce-cd is a separate SystemId, not a tg16 variant

**Decision:** PC Engine CD-ROM² games live under a dedicated `pce-cd`
`SystemId` in the frontend registry — separate sidebar entry, separate
theme (cyan-blue at 220°), separate per-system settings file, separate
per-system bindings file. Cart games (`.pce`) stay under `tg16`. The two
systems share the libretro core .dll and the entire input pipeline
(`bindings.rs::bit_for / buttons_for / defaults_for / to_libretro_bits`
all dispatch `tg16` and `pce-cd` to the same PCE_BUTTONS table and remap).

**Why:**

1. **User-visible split.** Cart and CD games have very different shelves
   in the real world (HuCard collectors vs. CD collectors typically don't
   overlap; CD games are 5-10× more storage; CD games carry CDDA + FMV +
   redbook-audio metadata that carts don't). A unified `tg16` system
   page would force these into one bucket and lose that signal.
2. **Per-system settings make sense.** A user might want Phosphor shader
   on TG-16 carts (CRT scanlines on chiptune-era PCE games) but a clean
   passthrough on TG-CD FMVs. Or a different default core. The per-system
   settings inheritance chain (OA → per-system → per-game) already exists;
   splitting into two systems means users can configure them independently
   without overrides.
3. **`oa_core::SystemId::PceCdRom2` already existed** as a Rust enum
   variant, and `parse_system_id` already mapped `"pce-cd"` to it — the
   plumbing was wired before the user-facing split shipped.
4. **Shared core .dll + shared input bindings minimize duplication.**
   The split costs three things: a registry entry, a CSS palette block,
   a one-shot SQLite migration. It does NOT cost a new core build, a new
   button table, or a new key-mapping UI.

**Considered and rejected:**

- **Keep CD games under tg16; surface "Cart only / CD only" as a filter
  view inside the existing system page.** Simplest, but loses the per-
  system settings independence and the at-a-glance library color cue.
- **Nested "PC Engine family" tree in the left sidebar** (tg16 → pce-cd
  → sgx). Cleaner long-term, but Phase 2.6 left nesting deferred — the
  flat sidebar order is what we have today. Revisit when SuperGrafx or
  another sibling system ships and the family tree pays for itself.
- **Per-game core override only (no SystemId split).** The override
  exists and works, but a user shouldn't have to set it per-game just to
  pick the right core for their entire CD collection.

**Migration:** Library DB v4→v5 retags existing `tg16` rows whose
`file_path` or `archive_inner_path` ends in a CD container extension
(`.cue` / `.chd` / `.ccd` / `.toc` / `.m3u` / `.iso`) as `pce-cd`. The
`v4_to_v5_retags_cd_games_to_pce_cd` test in `library_db.rs` covers the
case matrix including the trick case of a `.pce` file whose path
contains the substring "cue".
