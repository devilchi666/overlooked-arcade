# channelf Session Log

Per-core Shipped / Almost / Next log for Fairchild Channel F. Project-wide log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-19 — Phase 0 onboarding

- **Shipped:** `oa_core::SystemId::ChannelF`. `bindings.rs::channelf` 9-button module (4-axis plunger as D-pad + FIRE + 4 console switches MODE/TIME/START/HOLD). `default_core_dll_for_system("channelf") → "freechaf_libretro.dll"`. `media` + `rom_hashes` arms. Frontend `systemThemes.channelf` (extension `["chf"]`, portrait 3/4, crt-lite) + `[data-system="channelf"]` block (cedar-brown 25° / L=0.45 / C=0.06). Per-core docs.
- **Almost:** Phase 1 operator validation. Video Whizball, Spitfire good test cases. Tiny library overall (~26 titles).
- **Next:** Operator installs `freechaf_libretro.dll`, scans Channel F folder (or configures per-folder `*.bin → channelf` rule for older `.bin`-shaped dumps), launches a known-good ROM. The original 1976 wood-grain wedge gets a cedar-brown theme that intentionally rhymes with the 2600's yellow-pine brown — the two pioneer wood-grain consoles read as a family.
