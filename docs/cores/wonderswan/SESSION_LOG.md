# wonderswan Session Log

Per-core Shipped / Almost / Next log for Bandai WonderSwan. Project-wide log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-20 — Phase 0 onboarding

- **Shipped:** `bindings.rs::wonderswan` 7-button module (D-pad + A + B + START; the dual-physical-D-pad rotation is core-side per game header). `default_core_dll_for_system("wonderswan") → "mednafen_wswan_libretro.dll"`. `rom_hashes::libretro_dat_refs_for_system("wonderswan")` returns TWO DatRefs (WS + WS Color merged corpus via `fetch_and_parse_all`). Frontend `systemThemes.wonderswan` (extensions `["ws", "wsc"]`, portrait 3/4, crt-lite) + `[data-system="wonderswan"]` block (pearl lavender 305° / L=0.70 / C=0.14 — period-correct for the WS Color sherbet/pearl shell variants). Per-core docs.
- **Almost:** Phase 1 operator validation. Final Fantasy I (WSC), Klonoa, GunPey good test cases.
- **Next:** Operator installs `mednafen_wswan_libretro.dll`, scans WS folder, confirms pearl-lavender themed tiles, launches both a mono `.ws` and a color `.wsc` ROM. WSC-specific cover-sync gap documented as multi-repo follow-up (same shape as the `gb` ↔ GBC gap).
