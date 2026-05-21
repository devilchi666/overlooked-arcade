# RetroArch Feature Survey

**Date:** 2026-05-21
**Sources:**
- RetroArch headline feature catalogue (`docs.libretro.com/`)
- Top GitHub issues on `libretro/RetroArch` sorted by reactions (pages 1–6)
- Libretro forum top-all-time threads across the RetroArch / RetroArch Additions / Shaders / Cores / Overlays categories
- Libretro Bounty Discussion category
**Scope:** RetroArch-derived features worth considering for OA that are **not already** shipped, in `docs/NEXT.md`, in `docs/PARKING_LOT.md`, or covered by `launcher-landscape.md` / `launchbox-forum-feature-survey.md`.

Companion to those documents. `launcher-landscape.md` captures competitor *product* features; `launchbox-forum-feature-survey.md` captures LaunchBox community feature *requests*; this doc covers RetroArch — the dominant libretro-frontend incumbent.

RetroArch matters specifically because **OA is also a libretro frontend** (since the 2026-05-16 architecture pivot). Anything RetroArch does well is something an OA user is implicitly comparing us against. The features below are the ones where RetroArch sets the bar.

Same tags as the LaunchBox survey:
- **★** strong fit, premium-shell flavour matches OA pillars.
- **○** decent fit, worth queueing once HIGH backlog clears.
- **△** borderline; could go either way depending on operator interest.

---

## How OA already stacks up

For reference, here's what OA has already shipped that overlaps with RetroArch headline features. These items are *not* called out as gaps in the doc below.

- Save states + multi-slot UI + thumbnails (Phase 1.5)
- Rewind + TAS + video capture + memory inspector + milestones (Phase 4 A–F)
- Shader pipeline (Plain / Scanlines / CrtLite / Phosphor / LcdHandheld) + WGSL hot-reload + per-game / per-system override
- Bezel overlays (per-game + per-system)
- Soft-patching (IPS / UPS / BPS via `GameOverrides.patch_path` + `apps/oa-shell/src/patch.rs`)
- Disc swap / multi-disc M3U support
- Per-core, per-system, per-game option cascade (with dynamic visibility for SET_CORE_OPTIONS_DISPLAY)
- Input bindings UI per system
- Audio device picker
- In-app debug log viewer (`Help → Debug log…`)
- Direct-launch CLI
- Hash ROM identification (sha1 + libretro-database DAT match)
- Cheat-search infra (`apps/oa-shell/src/cheat_search.rs`)
- Soft-patching infra

Anything in the survey below is something OA does *not* yet do.

---

## Latency-reduction features (RetroArch's strongest cultural pillar)

RetroArch built a multi-year identity around input-latency engineering. Of every emulator frontend in existence, RetroArch is the only one with a coherent latency story, and it shows in the community's investment in CRT shaders + scanline sync + run-ahead techniques. This is the area where OA has the most to gain by matching — and the most credibility risk if we don't.

- **★ Run-Ahead (single-instance + two-instance).**
  RetroArch's flagship input-latency-reduction feature. From `docs.libretro.com/guides/runahead/`: the core's `retro_run` is invoked N frames ahead with audio/video disabled, save-state is captured, then the displayable frame is rendered. Two-instance mode runs a second core in parallel as a "leader" so the primary core never has to roll back. **Requires:** clean save states (every libretro core we ship supports this), sufficient CPU headroom (modern desktop is fine). User-configurable N (typically 1–4). Surfaces in Quick Menu → Latency. This is the single feature most-requested by speedrunners + rhythm-gamers + fighting-game players, and the most-cited reason to choose RetroArch over standalone emulators. **OA shipping run-ahead, with a polished UI to tune it, is a Tier-A unlock.** ~400 LOC against the existing `LibretroCore` save-state path.

- **★ Pre-Emptive Frames.**
  RetroArch's newer, lower-CPU alternative to run-ahead two-instance. Core advances state speculatively without rendering audio/video, then the "real" run picks up from the speculated state. Same latency benefit as run-ahead-single but ~half the CPU cost. Pairs with run-ahead; ship both with one toggle picking the strategy. ~200 LOC on top of run-ahead.

- **★ Auto-frame-delay (dynamic).**
  RetroArch's V-sync companion: dynamically adjust the gap between v-sync and game-frame start so the input poll is as late as possible (maximising frame-input freshness) without missing the v-blank deadline. Renderer-side feature, sits between OA's wgpu present chain and the libretro `retro_run` call. ~150 LOC; pairs with the GPU sync settings OA already exposes.

- **△ Beam Racing / Scanline Sync (`#6984`, "lagless VSYNC").**
  Sub-frame v-sync timing — present the framebuffer scanline-by-scanline so input → output latency is sub-16ms. RetroArch has had an open issue + active discussion for years (`#6984`); only a handful of implementations exist (the LibVR2 fork, RetroBat patches). **Why △:** the technique is fragile across GPU drivers, very platform-specific (works best on Vulkan + AMD/Intel; NVidia handling is finicky), and the benefit is small (~3-8ms) on top of run-ahead's 16-66ms. Worth tracking; not worth pursuing without a focused engineering investment.

- **○ Polling-mode customisation.**
  RetroArch exposes "Late" / "Early" / "Normal" input polling modes — controls when in the frame loop input is read, which affects perceived latency. ~30 LOC on `InputPoller`; surface as Quick Settings dropdown.

## Display + scanout quality features

CRT-correctness and HDR are the second major RetroArch cultural pillar — the libretro forum's `Shaders` category is the most active forum section by far (top thread "Were composite video colors that bad, in practice?" has 272 replies, 13.5k views). OA's WGSL pipeline + wgpu makes us natively capable of more than RetroArch ever was here.

- **★ Black Frame Insertion (BFI) shader pass.**
  RetroArch ships BFI at multiple refresh rates (#10754 is the open issue for higher-refresh BFI). Pattern: insert one black frame between each rendered frame on a 120Hz/144Hz/240Hz display → halves sample-and-hold motion blur, restores CRT-like motion clarity on LCDs. Needs a flag to detect display refresh + composit a black frame at every other present. Trivial WGSL pass + present-chain control. ~80 LOC. **Why ★:** CRT enthusiasts already running 144Hz panels treat this as table stakes.

- **★ HDR scanout (peak luminance / paper-white / tone mapping curve).**
  RetroArch has Vulkan-based HDR output with configurable peak luminance, paper-white, and tone mapping (BT.2020 colour space, ST.2084 transfer). wgpu supports HDR natively on DX12 + Vulkan + Metal. Pairs *beautifully* with the per-system ambient theming + the Phosphor / CrtLite shader presets — phosphor on a 1000-nit HDR display is genuinely transformative. ~250 LOC for the swap-chain + render-target plumbing, ~50 LOC for the UI sliders.

- **★ CRT SwitchRes (drive a real CRT).**
  RetroArch's most-loved niche feature: emit a 15kHz signal to a real CRT TV via dedicated capture cards / GroovyMAME-compatible setups. Picks pixel-perfect modeline per game (NES 60Hz, NTSC 60.098Hz, PAL 50Hz). Already inappropriate for vanilla desktop; the CRT cabinet cohort is small but vocal — and a CRT-friendly OA is unique among modern frontends. Implementable on Windows via `EnumDisplaySettings` + custom modeline injection. ~400 LOC for the basic case; Linux via DRM/KMS adds another ~300. **Why ★:** *if* OA wants to court the cabinet-builder cohort identified in `launchbox-forum-feature-survey.md` Tier D, this is the headline feature.

- **○ Underlay slot (image *behind* the game, not overlay).**
  RetroArch issue #15138. Today RetroArch composites bezels *over* the game frame; the underlay slot puts a backdrop *behind* the game (e.g. ambient room photo, era-appropriate desk wallpaper). OA's WGSL ambient-background hook in `launcher-landscape.md` §S2 covers this in spirit for menus; extending to in-game underlays is one more layer in the existing wgpu composite chain. ~60 LOC.

- **○ Per-game custom viewport (max width/height for integer scaling).**
  RetroArch issue #10024. Currently integer-scale picks max integer multiple that fits the screen. Users want to **clamp** integer scaling at a smaller max (e.g. "never scale 240p past 4×") for fixed-position bezel alignment on multi-monitor setups. ~50 LOC against existing scaling-mode resolution.

- **○ Custom modeline + refresh-rate per system / per game.**
  RetroArch lets users pick a specific refresh-rate target per content (50Hz for PAL, 60.098 for NTSC). OA's existing Display chain handles aspect + overscan + scaling; refresh override would slot in alongside. Needed when running on a VRR display that doesn't quite match the original 60.098 timing. ~80 LOC against `system_settings`.

## RetroAchievements deep integration

The RetroArch user base treats RetroAchievements (RA) as a core part of the experience — multiple top GitHub issues, most recurring forum thread cluster. `launcher-landscape.md` §C1 mentions deep RA integration as a "could." The deep dive surfaces specific asks:

- **★ Hardcore mode toggle + RetroAchievements login (the basic integration).**
  Phase-2 ship per §C1. Auto-login passthrough on launch, badge cache, mastery indicator on tile, per-system toggle (already noted in `launchbox-forum-feature-survey.md`).

- **★ Achievement notifications with captured screenshot + badge image (#11364, #14032).**
  When the user earns an achievement, the toast notification includes both the badge PNG *and* a screenshot of the game state at the unlock moment. Save the (badge + screenshot + timestamp) tuple to a per-user achievement gallery. The screenshot capture leverages OA's existing `video_capture.rs`. ~120 LOC.

- **★ Offline achievement queue + sync (#14718).**
  Allow earning achievements while offline; queue the unlock events to a local SQLite table; sync to RetroAchievements.org on next connectivity. RetroArch users have asked for this for ~3 years. ~150 LOC on top of the base RA integration.

- **○ Per-platform RetroAchievements toggle.**
  Users frequently want RA *off* for systems with junky achievement sets (often listed: Atari 2600, ColecoVision, some MAME). Per-system setting; one bool. ~10 LOC.

- **○ Achievement badge filter / "show only games with achievements I haven't earned."**
  Library filter chip + sort option. Sits on the existing tag/filter infrastructure recommended in the LaunchBox survey. ~30 LOC.

## Input layer (where RetroArch's depth shows)

- **★ Input mapper macros (`#8209`).**
  Combo bindings: one logical button trips a sequence of physical core inputs ("press A+B+Start over 3 frames"). Used by fighting-game players (combo execution), JRPG grinders (auto-attack), accessibility users (one-button → complex sequence). OA's existing `bindings.rs` cascade is per-physical-button; macros add a sequence layer on top. Storage: `macro_bindings(name, sequence_json)` table, per-system + per-game cascade. UI: macro editor with timing controls. ~250 LOC.

- **★ Lightgun-as-joystick + permanent lightgun routing (`#13425`).**
  Bind a lightgun (Sinden/GUN4IR/AimTrak) to act as a joystick for non-lightgun games. Inverse already exists in OA's POINTER infra (mouse-as-pointer for NDS/PSP/PS2). Lightgun-as-joystick lets a single peripheral cover the whole library. ~80 LOC against existing POINTER + analog routing.

- **★ Permanent controller-to-port assignment (`#12924`).**
  RetroArch struggles with the "I unplug my P2 controller for 5 minutes, when I plug it back in it's now P1" problem. Solution: stable per-device-UID port assignments persisted on disconnect/reconnect. SDL2 + gilrs both expose stable device UIDs. ~100 LOC against `InputPoller`.

- **○ Save Hotkeys per Autoconfig Profile (`#16112`).**
  Hotkey bindings ("Quick Save", "Quick Load", "Fullscreen") today live globally in OA — RetroArch users want them per controller-profile (your Xbox controller has hotkeys mapped one way, your arcade stick another). Storage: extend hotkey settings to be keyed by autoconfig profile. ~80 LOC.

- **○ "Delete override" UI in-app (`#11442`).**
  Once per-game / per-system overrides accrue, users want a UI to inspect + delete them without filesystem hunting. OA's `GameOverrides` row + the per-game settings drawer + a "reset to system default" button + "reset to core default" button covers the same need. ~50 LOC.

- **○ Per-system + per-game input polling frequency.**
  Some cores (NES) need 60Hz polling; arcade racers want 1000Hz. RetroArch exposes this per content. ~40 LOC against `InputPoller`.

- **△ On-screen virtual keyboard with copy/paste (`#10244`).**
  Useful for touch installs and gamepad-only navigation. Not relevant for desktop with keyboard attached; very relevant if OA ever ships a 10-foot / touch UX (likely Phase 3+ when handheld / Steam-Deck-class hardware support arrives).

- **△ Native multi-device input arbitration (XInput + DInput + Bluetooth + gilrs).**
  RetroArch has long-running open issues around Bluetooth controllers with duplicate VID/PID (#13520). gilrs handles most of this already; document the known-good behaviour rather than fixing what isn't broken.

## Save / load / state management

- **★ Multiple save slots with descriptive names (`#15441`).**
  OA's Phase 1.5 ships multi-slot save states with thumbnails. The RetroArch ask is: name your slots ("before final boss", "puzzle 7 solved") rather than slot 0–9 anonymous. Storage: add `name TEXT` column on the save-state table. UI: inline-rename on the save-state list. ~50 LOC.

- **★ Save state shaders (record + replay the shader preset alongside the state).**
  Niche but premium-feel: a save state can carry "this state was authored with CrtLite + bezel B" and restoring it reapplies that shader preset. Useful for cinematic streamers / TAS authors. ~100 LOC against existing save-state metadata header.

- **○ Cloud save sync (`#6875`, `#16566` done for iCloud, `#17686` done for Switch).**
  `launcher-landscape.md` §C7 lists this as a "could." The RetroArch GitHub data confirms it's been the #2-reaction issue on the entire project for 6+ years. Pattern: bundle SRAM + save-state + override config into a per-game "save bundle," push to a user-configured backend (S3-compatible, WebDAV, GDrive, OneDrive, iCloud-Drive on macOS, rclone-style remote). Defer-until-Phase-4+ at minimum but worth named on the roadmap because the demand is so concentrated.

- **○ Save state increment / max-states polish (`#16693`).**
  When a slot fills, auto-increment vs overwrite-oldest needs explicit UX. RetroArch's behaviour is muddy. OA can ship a clean policy from the start.

## Game-on-screen overlay & HUD

- **★ Low-battery overlay (`#14536`).**
  Subtle on-screen notification when the connected gamepad battery drops below 20% / 10% / critical. Cross-platform via gilrs `Gamepad::power_info`. ~20 LOC + an overlay slot in the existing renderer composite chain.

- **○ Statistics overlay (FPS + frame time + audio latency + run-ahead frames).**
  OA already has `PerformanceHud.tsx`; extending with RetroArch-style fields (audio buffer fill, run-ahead delta, polling latency) once run-ahead ships. ~30 LOC on top.

- **○ Achievement / save-state / disc-swap "global toast" stream.**
  RetroArch has a single toast queue for transient OSD notifications. OA likely has something similar; explicit feature is "every notification surfaces in the same place with the same visual language." Worth auditing once RA + low-battery features come together.

## AI Service (translation + accessibility)

- **★ AI Service: OCR + translate + overlay (per `docs.libretro.com/guides/ai-service/`).**
  RetroArch's most-overlooked-but-magical feature. Hotkey captures the current frame, posts to a user-configurable endpoint (`http://localhost:4404` for a local OCR/translate service, or a cloud endpoint), receives back translated text + overlays it on screen. **Killer use case for OA specifically:** the MSX / PCE / WSC / FDS Japan-only libraries (which are most of our "overlooked" lineup). A Japanese-only RPG that magic-translates to English on demand is the single most differentiating feature OA could ship for the otaku-retro crowd. Compatible with VGTranslate / ZTranslate / sugoi-toolkit endpoints. ~250 LOC: hotkey + frame-grab + HTTP POST + overlay composit + caching. **Phase 4+ but flag as strategic.**

- **○ Text-to-speech (narrator mode) for accessibility.**
  Same AI Service endpoint, different output mode — speak the OCR'd text via system TTS. For visually-impaired gamers (the `launchbox-forum-feature-survey.md` accessibility ask). Add Windows SAPI / macOS AVSpeech / Linux speech-dispatcher backends. ~80 LOC on top of AI Service.

## Cheats

- **★ First-class cheat code path (system-agnostic).**
  Already in `docs/NEXT.md` DEFERRED section ("System-agnostic cheat code path ~300 lines"). RetroArch's implementation is the reference: per-game `.cht` file with name + code + enabled flag per cheat; UI for enabling/disabling; Game Genie + Action Replay format support per system. Storage: extend `GameOverrides` with `cheats_json` column, parse at launch, feed to libretro's `retro_cheat_set` / `retro_cheat_reset` API. ~300 LOC matches the estimate; this becomes more visible once the survey-driven Tier-A queue clears.

- **○ Cheat search infra (real-time RAM scanning to author cheats).**
  Partially exists in `apps/oa-shell/src/cheat_search.rs`. RetroArch ships a similar live-scan tool. Once the cheat-load path lands, the existing search code becomes the authoring counterpart.

## Soft-patching extensions

OA's existing soft-patch path handles IPS / UPS / BPS at launch via `GameOverrides.patch_path`. RetroArch's deeper patterns worth lifting:

- **○ Multiple patches stacked per game.**
  E.g. "graphics restoration patch" + "translation patch" applied in sequence. RetroArch supports a single patch; OA could support an ordered list. Storage: change `patch_path` to `patch_paths_json`. ~40 LOC.

- **○ Patch discovery from a `<exe_dir>/patches/<system>/<game>.ips` convention.**
  Auto-detect a sibling patch file without explicit per-game wiring. ~30 LOC.

- **△ Live-toggle patches without restart.**
  RetroArch can't do this; we probably can't either (cores load patched ROM bytes at boot). Listed for completeness; defer.

## Netplay

- **△ Netplay (rollback + spectator + lobby).**
  RetroArch's netplay is its most ambitious feature: rollback-based, with spectator mode, custom relay servers (#8124), WiFi Direct local play (#8124), hybrid netpacket+rollback (#18897). Worth a serious investment if OA wants to be a *playing-with-friends* frontend rather than a solo-curation one. ~2000+ LOC for a credible implementation. **Borderline** because the moderation + lobby surface is a substantial product on top, and it's not aligned with OA's "premium gift to the retro community" framing (which is implicitly solo / archival). Re-evaluate if a community-organising operator picks up OA.

- **△ Cross-emulator deterministic save-state for netplay.**
  Even within RetroArch, netplay only works core-to-core (same core version, same options). Documenting this constraint is more valuable than building elaborate cross-version support.

## Quality-of-life polish from RetroArch

- **○ Show core version + author + license in the in-app core picker (`#5492`).**
  OA's Core Installer + buildbot catalog UI lists cores. Adding version / author / build date / license to each entry, and surfacing on-screen which version is currently loaded, is one of RetroArch users' most-cited "should be obvious but isn't" gripes. ~40 LOC.

- **○ "Reset to default" buttons throughout settings.**
  RetroArch has these per-section. OA's three-tier cascade (OA / system / game) already conceptually supports this — explicit reset buttons on each tier make the cascade visible. ~80 LOC of UI.

- **○ Searchable core options.**
  When a core has 200+ options (PCSX2, melonDS), scrolling is hostile. Add a search field at the top of the per-system / per-game core-options panel. Already feasible with the existing options-list component + a `createMemo` filter. ~30 LOC.

- **○ Reduce extracted-files count (`#12141`).**
  Niche packaging concern: RetroArch's installer drops thousands of files in `assets/`. OA can be deliberate about installer footprint from the start. Mostly a packaging discipline issue rather than a feature.

- **○ "Reload last content" hotkey.**
  RetroArch has "Restart" + "Load Last" entries in its main menu. OA can ship the same: one hotkey relaunches the most-recently-played game. ~20 LOC against the existing recent-games list.

- **△ Lua scripting extensions (`#6454`).**
  RetroArch's perennial request — Lua for memory-watch widgets, custom OSD, cheat authoring. The `launcher-landscape.md` §S18/S19 plugin tiers cover this conceptually (declarative TS plugins + native Rust plugins). Lua specifically is *not* the right primitive for OA — TS-in-WebView is. Keep S18/S19 wording; ignore the Lua framing.

## Recording / streaming

OA has `video_capture.rs`. RetroArch's deeper pattern set:

- **○ OBS-style scene helpers (windowed-with-transparent-bezel for greenscreen).**
  Useful for content creators. Rendering target with optional alpha + chroma-key border. ~80 LOC of renderer config; pairs with the existing capture path.

- **○ Replay buffer (last 60s rolling).**
  Already noted in `launchbox-forum-feature-survey.md`. RetroArch doesn't ship this; OBS does. Worth picking up on top of the existing capture infra.

- **△ RTMP streaming endpoint built in.**
  RetroArch supports streaming to Twitch / YouTube. Niche; defer to OBS coexistence.

## Theming / appearance

- **△ Multiple menu drivers (XMB / Ozone / MaterialUI / RGUI).**
  RetroArch's biggest UX miss — four menu drivers, each with their own asset pipeline, none of which are great. OA's single-shell decision (`docs/RESEARCH/launcher-landscape.md` §10 anti-pattern: "Don't ship two UI shells") is correct. **Listed for completeness; explicit anti-feature.**

- **△ Quick Menu (in-game pause overlay) parity with RetroArch.**
  OA's Quick Settings (Phase 2.8.B) covers this. RetroArch's QM has deeper hierarchy (controls, options, achievements, cheats, save states, disc swap, restart) — OA's tab-strip approach is the right shape, just needs the breadth filled in as adjacent features land. Not a discrete feature; an ongoing surface.

- **△ Automatic dark/light theme detection (`#7131`).**
  Trivial via `prefers-color-scheme` media query. ~5 LOC but OA's premium-shell aesthetic is dark-only by design.

---

## Explicit non-scope (RetroArch-driven but rejected for OA)

- **Core requests (Cemu / Xemu / PCSX2 / OpenMW / etc.).**
  OA loads any libretro `.dll`. Users download the .dll they want; we don't ship most cores ourselves. These are *core* additions, not *frontend* features.
- **Platform ports (3DS / Vita / Wii / WiiU / Switch / iOS / Android).**
  OA is desktop-only at present. Cross-platform expansion is `launcher-landscape.md` Q4, not RetroArch-driven.
- **AppImage updater (`#18924`).**
  OA's distribution model is operator-controlled (installer + cores folder); not relevant.
- **Vulkan driver loading on Android (`#18143`).**
  N/A.
- **Network share / NAS / SMB / NFS / UPNP / FTP support (`#11773`, `#11518`).**
  OA reads from local paths. If a user wants NAS, they mount it as a drive at the OS level. The library scanner doesn't care whether `D:\Roms` is local or networked.
- **Multiple core directories (`#3237`).**
  OA fixes cores at `<exe_dir>/cores/`. The forum ask is for users juggling parallel RetroArch installs; OA's single-install model sidesteps it.
- **iOS deep links (`#16584`).**
  N/A until OA goes mobile.
- **AirPlay screen mirroring as secondary screen on iOS (`#15257`).**
  N/A.
- **Steam Deck-specific quirks (`#14264`, `#14524`, `#14608`).**
  Covered by general cross-platform investment in `launcher-landscape.md` Q4. No Deck-specific feature work until Deck-on-Linux Tauri builds are first-class.
- **Qt6 migration (`#16913`) / Wayland support (`#17310`).**
  RetroArch-internal framework choices; OA picked Tauri.
- **3DS bottom screen input (`#9722`).**
  N/A.
- **AdrenoTools / Android GPU driver loading.**
  N/A.

---

## Recommended next bites — RetroArch flavour

Ordered by impact-per-effort, leaning into where RetroArch sets the cultural bar:

### Tier A — premium-shell credibility (these are the ones that win RetroArch users)
1. **Run-Ahead (single-instance + two-instance).** *The* feature that defines RetroArch's identity. Without it, latency-sensitive players won't migrate. ~400 LOC.
2. **Black Frame Insertion shader pass.** Trivial implementation, large felt impact on 144Hz+ displays. ~80 LOC.
3. **HDR scanout + tone mapping.** wgpu native; turns OA's existing CRT/Phosphor shaders into showpieces on HDR displays. ~300 LOC.
4. **RetroAchievements integration (base) + offline queue + screenshot-in-toast.** Table stakes for the retro audience. ~500 LOC for the base + ~150 for offline + ~120 for capture-and-attach. Stagger across two phases.
5. **AI Service translation overlay.** Differentiator for OA's Japan-only library cohort (MSX / PCE / WSC / FDS / NEC PC-FX). ~250 LOC.

### Tier B — depth that RetroArch users notice
6. **Input mapper macros.** Combos as named bindings. ~250 LOC.
7. **Pre-Emptive Frames (run-ahead's lower-CPU cousin).** Ship on top of Tier-A's run-ahead. ~200 LOC.
8. **Auto-frame-delay (dynamic v-sync timing).** Pairs with run-ahead. ~150 LOC.
9. **Permanent controller-to-port assignment.** Fixes a years-old RetroArch complaint. ~100 LOC.
10. **First-class cheat code path** (already in `NEXT.md` DEFERRED; surface as feature). ~300 LOC.
11. **Low-battery gamepad overlay.** Small, sweet, surprisingly absent in RetroArch. ~20 LOC.
12. **Multiple named save-state slots.** Rename existing slots. ~50 LOC.
13. **Lightgun-as-joystick.** Extends existing POINTER infra. ~80 LOC.

### Tier C — polish + accessibility
14. **TTS narrator on top of AI Service** (visually impaired accessibility).
15. **Per-platform RetroAchievements toggle.**
16. **Reset-to-default buttons throughout settings.**
17. **Searchable core options panel.**
18. **Show core version / author / license** in the core picker.
19. **Save state shader-preset capture** (Easter-egg-tier polish).
20. **Multiple stacked soft-patches per game.**

### Tier D — the niche-but-legendary feature
21. **CRT SwitchRes** for real-CRT cabinet builders. The kind of feature that gets OA written up in the modding press. ~700 LOC across Windows + Linux. Only pursue if the cabinet-builder cohort identified in `launchbox-forum-feature-survey.md` materialises.

### Defer / monitor
- Netplay (rollback + lobby). Strategic question: solo-curation product or play-with-friends product? Current OA framing leans solo.
- Beam-racing / scanline-sync. Technically interesting; benefit-per-effort marginal after run-ahead lands.
- Cloud save sync. Demand is real, but implementation is heavy and OA can ride export/import migration tools (`launchbox-forum-feature-survey.md`) for the same use case at first.

---

## Cross-references

- `launcher-landscape.md` — competitor product analysis; RetroArch row in §2 table.
- `launchbox-forum-feature-survey.md` — LaunchBox community feature requests.
- `docs/NEXT.md` — cross-system priority queue.
- `docs/PARKING_LOT.md` — explicit deferrals with dates.
- `docs/DECISIONS.md` 2026-05-16 libretro pivot — the architectural decision that made RetroArch-feature-matching strategically important.

When a feature in this doc moves into active work, flip it on the per-core ROADMAP (cross-system features land on multiple ROADMAPs at once — see CLAUDE.md "ROADMAP hygiene").
