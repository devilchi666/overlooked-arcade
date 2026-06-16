# Research need — External / standalone emulators (command-line launching)

**Status:** Research IN PROGRESS. The scope (roster + per-emulator template)
is below. **Verified 2026-06-15** — all of section A's single-system alts
(Dolphin, PCSX2, DuckStation, PPSSPP, melonDS, mGBA, Flycast, Mesen 2, DeSmuME)
plus section-B headliners Cemu + RPCS3, all checked against official
docs/source; see "Verified batch" below. **9 profiles authored + MERGED to main**
(branch `feat/external-emulator-profiles`) for the section-A emulators whose OA
system id already exists.

**DEPTH ARC NOW PLANNED (2026-06-15):** the depth work was designed in a
dedicated planning session →
[PLANS/external-emulator-depth.md](../PLANS/external-emulator-depth.md)
(decisions ED1–ED6 in
[features/external-emulators/DECISIONS.md](../features/external-emulators/DECISIONS.md)).
This doc remains the research seed (roster + per-emulator CLI/quirks + per-OS
binary table). **Open-item resolution:** schema question #1 (per-system args for
ares/BizHawk) is **dissolved** — both auto-detect (verified below); question #2
(MAME content model) + the per-OS `binary_name` map are folded into the arc's
**Slice 1**; question #3 (section-B system-id wiring) is the arc's **Phase 2
Slice 5+**, riding the per-system-descriptor loader. Originally raised 2026-06-14
alongside Settings IA Slice 4 (External Emulators consolidation).

## Open schema questions — RESOLVED in the depth arc (2026-06-15)

The 9 shipped profiles all fit today's schema (one flat `launch_args_template`
+ `{content}` = full file path, shared across all `supported_systems`). Three
deferred emulators broke that assumption. Resolution per the depth arc:

1. **Per-system argument variation (ares/BizHawk) — DISSOLVED (verified
   2026-06-15).** Both auto-detect the system from the file: ares `--system` is
   explicitly *optional* ("useful when the system type cannot be auto-detected" —
   [ares README/docs](https://github.com/ares-emulator/ares)); BizHawk maps file
   extension → console on load
   ([BizHawk wiki](https://github.com/TASEmulators/BizHawk/wiki/WIP-Manual:-Loading-roms)).
   So both get a single positional `{content}` recipe like the other emulators —
   no `{system}` token, no `system_aliases` map, no one-profile-per-pair. The
   optional `--system` override is a **reserved seam** (documented, not built)
   for the rare ambiguous-extension case (ED4).
2. **Non-path content model (MAME) — DEFERRED at Slice-1 execution
   (2026-06-15).** MAME takes a short **rom-set name** + a configured `rompath`
   (`mame sf2`), not a file path; software-list titles use `-cart`/`-cdrm`/etc.
   `{content}` = full path doesn't fit. Slice 1 considered adding a `content_mode`
   enum (`path` | `rom_name`) but **deferred the standalone-MAME profile**: the
   enum alone is *not* a clean ~1-field add — it has no consumer without real
   content resolution (rom-set-name extraction from a path + `rompath` config +
   library-scanner changes), which is well beyond Slice 1. Shipping a dead field
   would be a band-aid. The **in-process MAME core already covers arcade**, so the
   standalone profile waits until that content-resolution work is scoped (its own
   slice). No `content_mode` field exists in the schema today.
3. **Section-B system-id wiring (Cemu/RPCS3/Switch/3DS/Vita/Xbox/PS4/Model 3 +
   Wii).** Each needs an OA system id + `config/systems/<id>/` descriptor +
   sidebar/metadata before its (often already CLI-verified) profile is useful.
   PS3 additionally needs directory-based content resolution (EBOOT.BIN inside a
   game folder, not one ROM file) and a firmware precondition. This is VL Phase
   D (new-system installer + wiring) territory.

## Why this matters

OA already runs the bulk of its library through in-process **libretro cores**.
But standalone emulators are valuable for **two distinct reasons**, and the
operator wants BOTH covered:

1. **Systems we CANNOT run via an in-process core** — high-end / modern consoles
   whose emulators don't ship a usable libretro core (Wii U, PS3, Switch, 3DS,
   PS Vita, Xbox 360, …). For these, an external launcher is the *only* path.
2. **Systems we DO support via cores, but where users may PREFER a standalone**
   — e.g. someone who wants real PCSX2 instead of the PS2 core, DuckStation
   instead of the PSX core, or standalone Dolphin for its richer config. Giving
   users the *option* is the point — OA is a host, not a walled garden.

## Philosophy — start simple, grow

The launcher infra (`Launcher` trait + `ExternalProcessLauncher` +
`config/emulators/<id>.yaml` profiles, VL Phase C2) already does **simple
command-line launching**: expand a template like `--batch --exec={content}`,
spawn the process, watch for exit, restore OA. That's the floor.

The roster research below feeds a growth path:

- **Now (Slice 4 + this research):** one YAML profile per emulator with the
  minimum to *launch a game from a path*. Bring-your-own binary.
- **Next:** richer per-emulator options (fullscreen/borderless, content-format
  quirks, per-game config, save/state passthrough).
- **Later:** pre-launch **scripts** + **plugin** hooks (e.g. RPCS3 firmware
  checks, PCSX2 per-game patches), and the one-click **install pipeline**
  (VL **Phase D**).

So the YAML schema + the research template should be designed to *accrete*
options over time, not be rewritten.

## Where the research lands (existing infra)

- Profiles: `config/emulators/<id>.yaml` consumed by
  `apps/oa-shell/src/emulator_profiles.rs` (D4 fields: id / displayName / vendor
  / officialDownloadUrl / binaryName / supportedSystems / launch template).
  Shipped pilot: `dolphin.yaml` (`--batch --exec={content}`, systems
  `[gamecube]`).
- Launch: `ExternalProcessLauncher` in `apps/oa-shell/src/launcher.rs`.
- Plan context: [PLANS/virtual-library-and-launcher-arc.md](../PLANS/virtual-library-and-launcher-arc.md)
  (Phase C shipped; **Phase D** = install pipeline) +
  [PLANS/launcher-abstraction.md](../PLANS/launcher-abstraction.md).
- UI home (Slice 4): Settings → **External Emulators**.

## Per-emulator research template

For each emulator, gather:

| Field | What to capture |
| --- | --- |
| **id / displayName / vendor** | Profile identity. |
| **Systems** | Which OA system id(s) it covers. |
| **Official download URL** | Source for the binary (Phase D installer later). |
| **License / distribution** | Redistributable? (drives Phase D). **Zero ROMs / zero BIOS, ever.** |
| **Binary name(s)** | Per-OS exe/app/AppImage (Win/macOS/Linux differ). |
| **Launch CLI — boot a game** | The exact arg template + how `{content}` is passed (positional? flag? quoting?). |
| **Fullscreen / borderless flag** | And whether it's persistent config vs per-launch. |
| **Exit behavior** | Clean quit on close? Needs a kill fallback? (we already graceful→5s→kill.) |
| **Content formats** | Accepted (iso/chd/rvz/pkg/folder/title-id/…); any extract-first needs. |
| **BIOS / firmware / keys** | Required? Where placed? (We never ship these.) |
| **Config location** | Per-user config dir (for future per-game config). |
| **Per-game config** | Supported? How invoked? |
| **Save / state** | Where saves live; any sync concern. |
| **Controller passthrough** | Does our minimize-OA-on-launch interfere? |
| **Headless / batch flags** | `--batch` / `--no-gui` style. |
| **Plugin / script hooks** | Pre-launch needs (firmware install, key check, patch). |
| **Known quirks** | Anything that breaks naive `spawn(path, [content])`. |

## Roster to research

> ⚠️ The **CLI columns below are DRAFT from prior knowledge and MUST be
> verified** against each emulator's current docs/`--help` — flags churn between
> releases and across OSes. This is the research, not the answer.

### A. Systems OA supports via cores — standalone alternatives to offer

| Emulator | Systems | CLI launch (DRAFT — verify) | Notes |
| --- | --- | --- | --- |
| **Dolphin** | GameCube (✅), Wii (no id) | `--batch --exec={content}` | ✅ **VERIFIED + shipped** (`dolphin.yaml`). No `--fullscreen` flag exists — fullscreen is `-C Dolphin.Display.Fullscreen=True` or persistent ini. Wii half has no OA system id. |
| **PCSX2** (Qt) | PS2 (✅) | `pcsx2-qt.exe -batch -fullscreen -- {content}` | ✅ **VERIFIED + shipped** (`pcsx2.yaml`). Draft was old-wx & wrong: binary is `pcsx2-qt.exe`, path **must** follow `--`. User-dumped BIOS required. |
| **DuckStation** (Qt) | PSX (✅) | `duckstation-qt-x64-ReleaseLTCG.exe -batch -fullscreen -- {content}` | ✅ **VERIFIED + shipped** (`duckstation.yaml`). **License CC BY-NC-ND** — never bundle/redistribute/ship configs; link official download only. User-dumped BIOS required. |
| **PPSSPP** | PSP (✅) | `PPSSPPWindows64.exe {content} --fullscreen --escape-exit --pause-menu-exit` | ✅ **VERIFIED + shipped** (`ppsspp.yaml`). Path positional; **exit flags mandatory** or it idles on its own menu. No BIOS (HLE). |
| **melonDS** | DS (`nds` ✅) | `melonDS.exe -f -b always {content}` | ✅ **VERIFIED + shipped** (`melonds.yaml`). Path positional; `-b always` forces direct boot; `-f` fullscreen. No BIOS (built-in FreeBIOS). |
| **mGBA** | GBA (✅), GB (✅), GBC (✅) | `mGBA.exe -f {content}` | ✅ **VERIFIED + shipped** (`mgba.yaml`). Qt GUI build; path positional; `-f` fullscreen. No BIOS (HLE). |
| **Flycast / Redream** | Dreamcast (✅) | `flycast.exe -config window:fullscreen=yes {content}` | ✅ **Flycast VERIFIED + shipped** (`flycast.yaml`). No fullscreen flag — use `-config section:key=value`. **No auto-exit** (process exits via pause-menu Exit; hotkey unbound by default). No BIOS (HLE default). Redream not yet researched (closed-source). |
| **Mesen** | NES/SNES/GB/GBC/GBA/SMS/GG/PCE/WS/Coleco (all ✅) | `Mesen.exe --fullscreen {content}` | ✅ **VERIFIED + shipped** (`mesen.yaml`, Mesen **2**). Multi-system but **auto-detects** (no `--system` flag) → single positional template. Use the **native** (non-.NET) build. Carts BIOS-free; PCE-CD/FDS need user BIOS (excluded). |
| **DeSmuME** | DS (`nds` ✅) | `DeSmuME.exe {content}` | ✅ **VERIFIED + shipped** (`desmume.yaml`). Alt to melonDS. Path positional; **no fullscreen flag**. Windows exe is version-stamped (`DeSmuME_x64.exe`). No BIOS (HLE). |
| **BizHawk** | Multi (TAS) | `EmuHawk.exe "{content}"` | ✅ **VERIFIED + shipped** (`bizhawk.yaml`, Slice 1). Maps file extension → console on load, so single positional template. Per-OS `binary_name` map with **macOS omitted** (no native build). 18 BIOS-free auto-detect systems; disc+BIOS (PSX/Saturn) excluded for now. |
| **ares** | Multi | `"{content}"` | ✅ **VERIFIED + shipped** (`ares.yaml`, Slice 1). Auto-detects from the file; `--system` is optional/fallback-only (the reserved seam, ED4). Single positional template, full per-OS `binary_name` map. 15 BIOS-free systems; ambiguous-extension (MSX `.rom`) + CD/BIOS systems excluded. |
| **standalone MAME** | Arcade (`mame` ✅) | `mame <romname>` | ⏸️ **DEFERRED (re-confirmed Slice 1, 2026-06-15) — content-model mismatch.** MAME takes a short **rom-set name** + a configured `rompath`, not a file path, so `{content}=<full path>` doesn't fit. The `content_mode` enum is not a clean 1-field add (needs real content resolution); in-process MAME core already covers arcade. Waits on its own slice. |

### B. Systems with NO usable in-process core — standalone REQUIRED

| Emulator | System | CLI launch (DRAFT — verify) | Notes |
| --- | --- | --- | --- |
| **Cemu** | Wii U | `Cemu.exe -g {content} -f` | ✅ **CLI VERIFIED 2026-06-15** — but **needs `wiiu` system id first** (see wiring notes). `{content}` = `.wua` (best, no keys) / `.wud` / `.wux` / the `.rpx` inside an extracted `code/` folder. **No auto-exit** (window-close = exit, not game-end). `keys.txt` user-supplied for encrypted content. |
| **RPCS3** | PS3 | `rpcs3.exe --no-gui {content}` (+ optional `--fullscreen`) | ✅ **CLI VERIFIED 2026-06-15** — but **needs `ps3` system id first** (see wiring notes). `{content}` = path to `EBOOT.BIN`. **Firmware (PS3UPDAT.PUP) is a hard prerequisite**, user-installed once via File → Install Firmware (never shipped/downloaded by OA). `--no-gui` exit can linger (track + reap PID). |
| **Ryujinx** | Switch | `"{content}"` | Firmware + keys (user-supplied; never shipped). Watch project-status churn. |
| **Lime3DS** (Citra successor) | 3DS | `"{content}"` | Citra discontinued; Lime3DS is the maintained fork. |
| **Vita3K** | PS Vita | `-r <title-id>` / `--content "{content}"` | Install-then-run model. |
| **Xenia** | Xbox 360 | `xenia.exe "{content}"` | Windows-centric. |
| **xemu** | Xbox (original) | `-dvd_path "{content}"` | Heavily config-driven; needs MCPX/BIOS (user-supplied). |
| **shadPS4** | PS4 | `"{content}"` (eboot.bin) | Early/experimental; track maturity. |
| **Supermodel** | Sega Model 3 (arcade) | `supermodel "{content}" -fullscreen` | Standalone only. |

(Lists are a starting point, not exhaustive — add as the community surfaces
options. New systems in section B need an OA system id + metadata + sidebar
entry before their profile is useful.)

## Verified batch — 2026-06-15

Verification pass: each CLI checked against the emulator's **official docs
and/or source command-line parser** (not forum hearsay). Two sub-batches done
2026-06-15: the high-value headliners (Dolphin/PCSX2/DuckStation/PPSSPP +
section-B Cemu/RPCS3), then the remaining single-system section-A alts
(melonDS/mGBA/Flycast/Mesen/DeSmuME). Per-emulator findings below include the
data the current single-field schema can't yet hold.

### Profiles authored (OA system id already exists) — 9 total

| Profile | System(s) | argv template (Windows) | BIOS/firmware | Notes |
| --- | --- | --- | --- | --- |
| `dolphin.yaml` | `gamecube` | `--batch --exec={content}` | none to launch | shipped pilot; confirmed correct. |
| `pcsx2.yaml` | `ps2` | `-batch -fullscreen -- {content}` | PS2 BIOS (user-dumped) | path **must** follow `--`. |
| `duckstation.yaml` | `psx` | `-batch -fullscreen -- {content}` | PS1 BIOS (user-dumped) | CC BY-NC-ND — no bundling/configs. |
| `ppsspp.yaml` | `psp` | `{content} --fullscreen --escape-exit --pause-menu-exit` | none (HLE) | exit flags mandatory; path positional. |
| `melonds.yaml` | `nds` | `-f -b always {content}` | none (FreeBIOS) | `-b always` forces direct boot. |
| `mgba.yaml` | `gba` `gb` `gbc` | `-f {content}` | none (HLE) | Qt GUI build; path positional. |
| `flycast.yaml` | `dreamcast` | `-config window:fullscreen=yes {content}` | none (HLE default) | no auto-exit; long-lived child. |
| `mesen.yaml` | `nes` `snes` `gb` `gbc` `gba` `sms` `gamegear` `tg16` `wonderswan` `coleco` | `--fullscreen {content}` | carts none; PCE-CD/FDS need user BIOS (excluded) | Mesen 2; auto-detects; native build. |
| `desmume.yaml` | `nds` | `{content}` | none (HLE) | no fullscreen flag; version-stamped exe. |

**Multi-profile-per-system** is now live (first time beyond Dolphin): `nds`
has melonDS + DeSmuME; `gb`/`gbc`/`gba` have mGBA + Mesen. `launchers.json`
holds one default per system; the rest are opt-in. Both UI surfaces already
handle this (`CoreLauncherEditor` lists all supporting profiles in one
dropdown; `ExternalEmulatorsSection` shows coherent per-profile selects) — no
code change needed.

### Per-OS binary names (schema-accretion data — NOW REPRESENTABLE, Slice 1)

**Update 2026-06-15 (Slice 1):** the schema now holds this — `binary_name`
accepts a `{ windows, macos, linux }` map (untagged `BinaryName` enum in
`emulator_profiles.rs`); a bare string still applies to every OS. The 9
existing profiles keep their single string; `ares.yaml`/`bizhawk.yaml` use the
map. The single `binary_name` is still only a soft warn-check (an OS the map
omits → `resolve()` returns `None` → check skipped). Verified names per OS:

| Emulator | Windows | macOS | Linux |
| --- | --- | --- | --- |
| Dolphin | `Dolphin.exe` | `Dolphin.app/Contents/MacOS/Dolphin` | `dolphin-emu` / flatpak `org.DolphinEmu.dolphin-emu` |
| PCSX2 | `pcsx2-qt.exe` | `PCSX2-vX.Y.Z.app` (version-stamped) | version-stamped AppImage / flatpak `net.pcsx2.PCSX2` |
| DuckStation | `duckstation-qt-x64-ReleaseLTCG.exe` | `DuckStation.app` | `DuckStation-x64.AppImage` |
| PPSSPP | `PPSSPPWindows64.exe` | `PPSSPP.app/Contents/MacOS/PPSSPP` | `PPSSPPSDL` / `PPSSPPQt` / flatpak `org.ppsspp.PPSSPP` |
| Cemu | `Cemu.exe` | `Cemu.app/Contents/MacOS/Cemu` (experimental) | `Cemu-*.AppImage` / flatpak `info.cemu.Cemu` |
| RPCS3 | `rpcs3.exe` | `RPCS3.app/Contents/MacOS/rpcs3` | `rpcs3-*_linux64.AppImage` / flatpak `net.rpcs3.RPCS3` |
| melonDS | `melonDS.exe` | `melonDS.app/Contents/MacOS/melonDS` | `melonDS` / flatpak `net.kuribo64.melonDS` |
| mGBA | `mGBA.exe` | `mGBA.app/Contents/MacOS/mGBA` | `mgba-qt` (fallback `mgba`) / flatpak `io.mgba.mGBA` |
| Flycast | `flycast.exe` | `Flycast.app/Contents/MacOS/Flycast` | `flycast` / flatpak `org.flycast.Flycast` |
| Mesen | `Mesen.exe` | `Mesen.app/Contents/MacOS/Mesen` | `Mesen` (native build; needs system SDL2) |
| DeSmuME | `DeSmuME_x64.exe` (version-stamped) | `DeSmuME.app` | `desmume` (GTK) / `desmume-cli` (SDL) |

Cross-OS note: **launch args are OS-agnostic** for all six (the same flags
compile into every platform build) — only the binary name/path differs. So a
per-OS `binary_name` map is the right schema extension; `launch_args_template`
can stay single. macOS `.app` bundles must spawn the inner Mach-O directly (not
`open`, which detaches the child and breaks exit-tracking).

### Quirks captured (beyond the table)

- **Dolphin** — `--batch` requires `--exec`; without `--batch` the process drops
  back to the GUI and never exits (breaks lifecycle tracking). No `--fullscreen`
  flag; use `-C Dolphin.Display.Fullscreen=True` or persisted ini.
- **PPSSPP** — without `--escape-exit`/`--pause-menu-exit` the process idles on
  its own menu after a game stops, so OA never restores. `--fullscreen` may
  persist into saved config (issue #15557). Linux: pass the `EBOOT.PBP` file,
  not its folder.
- **DuckStation** — exe name ≠ download asset name ≠ `duckstation`; spawn
  `duckstation-qt-x64-ReleaseLTCG.exe`. **License is the constraint**: CC
  BY-NC-ND forbids shipping the binary OR a pre-configured settings package —
  reference the user's own install + link official download only (this directly
  shapes Phase D: download-from-official-only, no repackaging).
- **PCSX2** — everything after `--` is the filename (pass the whole path as one
  argv element; no shell quoting). `-portable`/`-datapath` exist to pin config.
- **Cemu** — binary is capital-C `Cemu` (matters on case-sensitive Linux FS).
  Portable mode = a `portable` *folder* next to the exe (not `portable.txt`).
- **RPCS3** — games must be pre-installed/decrypted (the frontend passes an
  existing `EBOOT.BIN`, never a raw ISO/pkg). Firmware must be installed first.
- **melonDS** — default boot mode is `auto` (boots when an NDS rom is given),
  but a persisted GUI "boot directly = off" can drop to firmware; `-b always`
  hardens against that. Avoid `|` in paths (archive|member syntax).
- **mGBA** — options-first/ROM-last is the documented form; only one trailing
  positional is consumed. Binary casing differs by OS (Win/macOS `mGBA`, Linux
  `mgba`/`mgba-qt`). No kiosk auto-exit — rely on window-close.
- **Flycast** — `-config` overrides must precede the content path and use
  `section:key=value`. Windows is **always portable** (emu.cfg + `data/` next to
  `flycast.exe`) — the operator's binary must keep its `data/` folder beside it.
  No auto-exit at game end; Exit hotkey unbound by default.
- **Mesen** — use the **native** build (the ".NET build" needs .NET 8 installed;
  Linux/macOS native builds still need system SDL2). A first-run data-location
  wizard can block an automated spawn until dismissed. Multi-system via
  auto-detect, so one positional template suffices (no `--system`).
- **DeSmuME** — Windows exe is version/arch-stamped (`DeSmuME_0.9.13_x64.exe`),
  not a stable `DeSmuME.exe`; users rename it. Loader only WARNs on name
  mismatch so the operator's actual path wins. No CLI fullscreen flag exists.
  Two distinct Linux binaries (`desmume` GTK vs `desmume-cli` SDL).

### Section-B wiring needed before a profile is useful

Both verified section-B headliners are **blocked on OA system-id wiring** — the
CLI is known, but there's no system for the profile's `supported_systems` to
reference, and no sidebar/metadata home for the games. Per system, what's
needed before authoring `cemu.yaml` / `rpcs3.yaml`:

- **Cemu → new `wiiu` system id.** Needs: a `config/systems/wiiu/system.yaml`
  descriptor (display name "Wii U", manufacturer Nintendo, media/extension
  hints for `.wua`/`.wud`/`.wux`/`.rpx`), sidebar + metadata entry, and a
  content-format decision (which path form OA hands to `-g`: prefer `.wua`).
  No libretro core exists, so `launchers.json` would default `wiiu → cemu`
  with no in-process fallback. `keys.txt` is a user-supplied prerequisite
  (never shipped) — surface as a precondition, like a BIOS gate.
- **RPCS3 → new `ps3` system id.** Needs: a `config/systems/ps3/system.yaml`
  descriptor (display name "PlayStation 3", Sony), sidebar + metadata entry,
  and a content-resolution decision — PS3 "games" are **installed directories**
  (`…/USRDIR/EBOOT.BIN`), not single files, so OA's library scanner +
  `{content}` resolution must point at `EBOOT.BIN` within a game folder rather
  than treating each game as one ROM file. **Firmware (PS3UPDAT.PUP) is a hard,
  user-installed prerequisite** — the canonical "plugin/script hook later"
  case (detect-and-prompt; never ship/fetch the PUP).

The same gap applies to every other section-B system (Switch/3DS/Vita/Xbox
360/Xbox/PS4/Model 3) and to **Wii** (Dolphin's Wii half — no `wii` id today,
only `gamecube`): each needs a system id + descriptor + sidebar/metadata before
a profile helps. This is properly **VL Phase D** territory (new-system
installer + wiring), not something to bolt on ahead of it.

## Open questions for the research

1. **OS coverage** — ✅ **RESOLVED (Slice 1, 2026-06-15).** `binary_name` now
   accepts a per-OS `{ windows, macos, linux }` map; CLI args proved OS-agnostic
   for the verified batch, so `launch_args_template` stays single. A per-OS
   `launch` override remains an additive future option if a real case needs it.
2. **Content resolution** — some want a folder / title-id, not a file path; our
   `{content}` substitution + archived-entry handling must account for that
   (C2 already errors clearly on archived externals).
3. **Phase D legal posture** — which binaries are redistributable vs.
   download-from-official-only; **zero ROMs / zero BIOS / zero keys, ever.**
4. **Per-game config + plugins** — the seam to reserve in the YAML now so it
   doesn't need a rewrite when scripts/plugins land.

## Output of the research (when done)

- One `config/emulators/<id>.yaml` per emulator (verified launch template).
- This doc's roster tables filled in with verified CLI + quirks.
- A short note per section-B system on the OA system-id / metadata work needed.
