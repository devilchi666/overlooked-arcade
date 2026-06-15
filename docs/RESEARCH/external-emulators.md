# Research need — External / standalone emulators (command-line launching)

**Status:** Research NOT yet done. This doc scopes the research — the roster to
investigate + the per-emulator data we must gather — so a future session (or the
operator) can fill it in systematically. Raised 2026-06-14 alongside Settings IA
Slice 4 (External Emulators consolidation).

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
| **Dolphin** | GameCube, Wii | `--batch --exec="{content}"` | ✅ shipped pilot (`dolphin.yaml`). |
| **PCSX2** (Qt) | PS2 | `-batch -fullscreen "{content}"` | More mature than the PS2 core for many titles. |
| **DuckStation** (Qt) | PSX | `-batch -fullscreen -- "{content}"` | Core exists (swanstation); standalone more featured. |
| **PPSSPP** | PSP | `"{content}" --fullscreen` | Core exists. |
| **melonDS** | DS | `"{content}"` | Core exists. |
| **mGBA** | GBA, GB/GBC | `"{content}" -f` | Core exists. |
| **Flycast / Redream** | Dreamcast | flycast `"{content}"` · redream `"{content}"` | Cores exist; Redream is closed-source. |
| **Mesen** | NES, SNES, GB | `"{content}"` | Core exists. |
| **DeSmuME** | DS | `"{content}"` | Alt to melonDS. |
| **BizHawk** | Multi (TAS) | `EmuHawk.exe "{content}"` | Multi-system; TAS/tooling audience. |
| **ares** | Multi | `--system <sys> "{content}"` | Multi-system; needs system disambiguation. |
| **standalone MAME** | Arcade | `mame <romname>` | We run MAME modules via cores; standalone for the long tail. |

### B. Systems with NO usable in-process core — standalone REQUIRED

| Emulator | System | CLI launch (DRAFT — verify) | Notes |
| --- | --- | --- | --- |
| **Cemu** | Wii U | `-g "{content}"` (`-f` fullscreen) | New system id needed. |
| **RPCS3** | PS3 | `--no-gui "{content}"` (EBOOT.BIN / game folder) | Firmware install step (plugin/script later). |
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

## Open questions for the research

1. **OS coverage** — Win/macOS/Linux binaries + CLI differ; do profiles need
   per-OS `binary_name` + `launch` (the schema should allow it).
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
