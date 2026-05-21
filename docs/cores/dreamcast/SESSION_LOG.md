# dreamcast Session Log

---

## 2026-05-20 — Phase 0 onboarding (Sega family completion)

- **Shipped:** SystemId variant (Dreamcast), parse_system_id arm,
  `bindings.rs::dreamcast` module (11-button: d-pad + A/B/X/Y face
  diamond + L/R analog triggers + START; no SELECT — DC pad doesn't
  have one). Default core Flycast. Media + rom_hashes arms
  (Sega_-_Dreamcast thumbnails repo; NO_DAT_SYSTEMS for GD-ROM CD
  images). `check_dreamcast_bios` + `DREAMCAST_BIOS_KNOWN_HASHES`
  table (4 entries: dc_boot.bin universal + dc_flash.bin US/JP/EU
  variants). **CD-launch BIOS dispatch arm now covers 8 CD-shape
  systems** (pce-cd / segacd / saturn / psx / neocd / 3do / pcfx /
  dreamcast). CSS theme: DC orange swirl `oklch(0.55 0.27 32)` —
  highest chroma in the warm zone; period-correct to 9/9/99 launch
  marketing + Dreamcast spiral logo. Per-core docs scaffold.
- **Wave 2 (Sega family completion) COMPLETE.** Combined with the
  earlier sms / gamegear / genesis / segacd / sega32x / saturn
  onboardings, OA now hosts all 7 Sega home consoles + their
  expansion hardware. Six of those seven shipped in the 2026-05-19
  / 2026-05-20 push.
- **Almost:** Phase 1 operator validation.
- **Next:** Operator drops `flycast_libretro.dll` + `dc_boot.bin` +
  a regional `dc_flash.bin`, marks a DC folder via Import Wizard,
  launches Sonic Adventure / Crazy Taxi / Soulcalibur. The first
  test: LeftStick should drive Sonic through the Station Square
  hub smoothly via the analog input infra shipped earlier today.
