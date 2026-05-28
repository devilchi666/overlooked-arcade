// Reusable settings section bodies for Retroverse-UI Phase C1.
//
// Each section component renders one logical settings surface without
// the Dialog wrapper. SettingsPage embeds these directly into its
// center pane (one per category); the legacy modal SettingsDialogs.tsx
// can later switch to importing from here too (deferred — the parallel-
// file approach keeps Phase C1 risk low).
//
// Section content + behaviour is bit-for-bit identical to the
// corresponding DialogSection blocks in SettingsDialogs.tsx — same
// SettingRow inputs, same store bindings, same descriptions, same
// createResource queries. Only the outer wrapper differs (Card vs
// DialogSection — both visually similar).
//
// See docs/PLANS/settings-tab-retroverse.md for the design + the
// rollout plan at docs/PLANS/retroverse-ui-rollout.md.

import {
  createMemo,
  createResource,
  For,
  Show,
  type Component,
  type JSX,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import CoresPage from "./CoresPage";
import SettingRow from "./SettingRow";
import {
  SCALING_MODE_LABELS,
  SCALING_OPTIONS,
  WINDOW_MODE_LABELS,
  WINDOW_OPTIONS,
  type AudioDeviceInfo,
  type ControllerNavSource,
  type MonitorInfo,
  type ScalingMode,
  type SettingsStore,
  type WindowMode,
} from "../settings/store";
import { shaderPresets, shaderPresetLabel } from "../settings/shader_presets";

const REWIND_INTERVAL_OPTIONS: readonly number[] = [1, 2, 3, 6, 10, 15, 30];
const REWIND_BUFFER_OPTIONS: readonly number[] = [8, 16, 32, 64, 128, 256, 512];

function monitorLabel(m: MonitorInfo): string {
  const name = m.name?.trim() || `Monitor ${m.index + 1}`;
  return `${name} (${m.width}×${m.height})`;
}

// Light card wrapper used to group rows inside the SettingsPage center
// pane. Cheap visual parity with DialogSection without depending on
// the Dialog primitive. Title is optional — a card with no title is
// just a rounded container.
const SettingsCard: Component<{
  title?: string;
  description?: string;
  children: JSX.Element;
}> = (props) => {
  return (
    <section class="rounded-xl border border-white/10 bg-white/[0.02] p-5">
      <Show when={props.title}>
        <header class="mb-3">
          <h3 class="text-[0.7rem] font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink)">
            {props.title}
          </h3>
          <Show when={props.description}>
            <p class="mt-1 text-[0.75rem] leading-relaxed text-(--color-oa-ink-dim)">
              {props.description}
            </p>
          </Show>
        </header>
      </Show>
      <div class="flex flex-col gap-3">{props.children}</div>
    </section>
  );
};

// --- Display (scaling / window / run-ahead) ----------------------------

export const DisplayBaseSettings: Component<{ settings: SettingsStore }> = (props) => {
  const [monitors] = createResource(async (): Promise<MonitorInfo[]> => {
    try {
      return await invoke<MonitorInfo[]>("list_monitors");
    } catch {
      return [];
    }
  });

  const scalingOptions = SCALING_OPTIONS.map((m) => ({
    value: m,
    label: SCALING_MODE_LABELS[m],
  }));
  const windowOptions = WINDOW_OPTIONS.map((m) => ({
    value: m,
    label: WINDOW_MODE_LABELS[m],
  }));
  const monitorOptions = createMemo(() => [
    { value: "current", label: "Current monitor" },
    ...(monitors() ?? []).map((m) => ({
      value: String(m.index),
      label: monitorLabel(m),
    })),
  ]);

  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="Scaling">
        <SettingRow
          label="Scaling mode"
          inherited={null}
          overridden={false}
          select={{
            value: props.settings.scalingMode(),
            options: scalingOptions,
            onChange: (v) => props.settings.setScalingMode(v as ScalingMode),
          }}
        />
      </SettingsCard>

      <SettingsCard title="Window">
        <SettingRow
          label="Window mode"
          inherited={null}
          overridden={false}
          select={{
            value: props.settings.windowMode(),
            options: windowOptions,
            onChange: (v) => props.settings.setWindowMode(v as WindowMode),
          }}
        />
        <SettingRow
          label="Monitor"
          hint="For borderless"
          inherited={null}
          overridden={false}
          select={{
            value:
              props.settings.monitorIndex() === null
                ? "current"
                : String(props.settings.monitorIndex()),
            options: monitorOptions(),
            onChange: (v) =>
              props.settings.setMonitorIndex(v === "current" ? null : Number(v)),
          }}
        />
      </SettingsCard>

      <SettingsCard title="Run-ahead">
        <SettingRow
          label="Run-ahead frames"
          inherited={null}
          overridden={false}
          slider={{
            min: 0,
            max: 5,
            step: 1,
            value: props.settings.runAheadFrames(),
            format: (v) => (v === 0 ? "off" : `+${v}f`),
            onInput: (v) => {
              if (Number.isInteger(v)) props.settings.setRunAheadFrames(v);
            },
          }}
          description="Reduces perceived input latency by N frames. Each costs one save_state + one run_frame + one load_state per frame. Skipped during scrub / TAS / pause."
        />
      </SettingsCard>
    </div>
  );
};

// --- Per-system UI -----------------------------------------------------

export const PerSystemUiSettings: Component<{ settings: SettingsStore }> = (props) => {
  return (
    <div class="flex flex-col gap-4">
      <SettingsCard
        title="Per-system experiences"
        description="Each system in your library can feel like its own mini-experience — per-system audio, boot animations, tile flourishes, backgrounds. Disable for a uniform plain library across every system. Stage 1 of the per-system-ui pipelined arc; later stages add per-system layout + behavior."
      >
        <SettingRow
          label="Enabled"
          inherited={null}
          overridden={false}
          toggle={{
            checked: props.settings.perSystemUiEnabled(),
            onChange: (v) => props.settings.setPerSystemUiEnabled(v),
          }}
          description="When off, every system shares one neutral library look — no per-system audio, no boot animations, no per-system flourishes. Cover art and tiles still show; only the per-system character disappears."
        />
        <Show when={props.settings.perSystemUiEnabled()}>
          <SettingRow
            label="Boot animations"
            inherited={null}
            overridden={false}
            toggle={{
              checked: props.settings.bootAnimationsEnabled(),
              onChange: (v) => props.settings.setBootAnimationsEnabled(v),
            }}
            description="Brief overlay that plays when you enter a system from the sidebar — tints the library with the system's accent color for about a second. Disable for instant transitions with no overlay; re-enable to restore the full animation. The OS reduce-motion preference shortens the overlay to a 200 ms fade independently when active."
          />
        </Show>
      </SettingsCard>
    </div>
  );
};

// --- Controller navigation --------------------------------------------

export const ControllerNavSettings: Component<{ settings: SettingsStore }> = (props) => {
  return (
    <div class="flex flex-col gap-4">
      <SettingsCard
        title="Controller navigation"
        description="Drive the library and menus with a gamepad. DPad or left stick moves focus; A activates, B cancels, X opens context menus, Y opens details, shoulder bumpers switch between panels."
      >
        <SettingRow
          label="Enabled"
          inherited={null}
          overridden={false}
          toggle={{
            checked: props.settings.controllerNavEnabled(),
            onChange: (v) => props.settings.setControllerNavEnabled(v),
          }}
        />
        <SettingRow
          label="Navigation source"
          inherited={null}
          overridden={false}
          select={{
            value: props.settings.controllerNavSource(),
            options: [
              { value: "both", label: "DPad + left stick" },
              { value: "dpad", label: "DPad only" },
              { value: "stick-left", label: "Left stick only" },
            ],
            onChange: (v) => props.settings.setControllerNavSource(v as ControllerNavSource),
          }}
        />
        <SettingRow
          label="Swap A and B"
          hint="Nintendo layout"
          inherited={null}
          overridden={false}
          toggle={{
            checked: props.settings.controllerNavSwapAB(),
            onChange: (v) => props.settings.setControllerNavSwapAB(v),
          }}
          description="When on, B confirms and A cancels — matches SNES/N64/DS-era Nintendo conventions."
        />
        <SettingRow
          label="Animation budget"
          inherited={null}
          overridden={false}
          select={{
            value: String(props.settings.controllerNavAnimationMs()),
            options: [
              { value: "0", label: "Snappy (no animation)" },
              { value: "120", label: "Subtle (120 ms)" },
              { value: "250", label: "Animated (250 ms)" },
            ],
            onChange: (v) => {
              const ms = Number(v);
              if (Number.isFinite(ms)) props.settings.setControllerNavAnimationMs(ms);
            },
          }}
          description="How long the focus ring transitions when moving between elements."
        />
      </SettingsCard>
    </div>
  );
};

// --- Experimental (hosts the Retroverse master toggle) ----------------

export const ExperimentalSettings: Component<{ settings: SettingsStore }> = (props) => {
  return (
    <div class="flex flex-col gap-4">
      <SettingsCard
        title="Experimental"
        description="Preview-quality features still under active development. Safe to toggle; defaults preserve today's behavior. See docs/PLANS/retroverse-ui-rollout.md for the rollout plan."
      >
        <SettingRow
          label="Retroverse UI"
          inherited={null}
          overridden={false}
          toggle={{
            checked: props.settings.experimentalRetroverseUi(),
            onChange: (v) => props.settings.setExperimentalRetroverseUi(v),
          }}
          description="Top-toolbar tab IA (HOME / LIBRARY / COLLECTIONS / PLAY NOW / DISCOVER / SETTINGS) replacing today's sidebar-driven layout. Flipping this OFF returns to the legacy Shell layout immediately — no restart required."
        />
      </SettingsCard>
    </div>
  );
};

// --- Audio --------------------------------------------------------------

export const AudioSettings: Component<{ settings: SettingsStore }> = (props) => {
  const [audioDevices] = createResource(async (): Promise<AudioDeviceInfo[]> => {
    try {
      return await invoke<AudioDeviceInfo[]>("list_audio_devices");
    } catch {
      return [];
    }
  });

  const deviceOptions = createMemo(() => [
    { value: "__default__", label: "System default" },
    ...(audioDevices() ?? []).map((d) => ({
      value: d.name,
      label: `${d.name}${d.isDefault ? " (default)" : ""}`,
    })),
  ]);

  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="Output device">
        <SettingRow
          label="Output device"
          inherited={null}
          overridden={false}
          select={{
            value: props.settings.audioDevice() ?? "__default__",
            options: deviceOptions(),
            onChange: (v) => props.settings.setAudioDevice(v === "__default__" ? null : v),
          }}
          description="Takes effect immediately — the running stream swaps in place."
        />
      </SettingsCard>
    </div>
  );
};

// --- Gameplay (rewind) -------------------------------------------------

export const GameplaySettings: Component<{ settings: SettingsStore }> = (props) => {
  const intervalOptions = REWIND_INTERVAL_OPTIONS.map((n) => ({
    value: String(n),
    label: `${n === 1 ? "Every frame" : `Every ${n} frames`} (~${Math.round((n / 60) * 1000)} ms at 60 fps)`,
  }));
  const bufferOptions = REWIND_BUFFER_OPTIONS.map((mb) => ({
    value: String(mb),
    label: `${mb} MB`,
  }));

  return (
    <div class="flex flex-col gap-4">
      <SettingsCard
        title="Rewind"
        description="Hold Backspace during gameplay to walk emulation backwards. Snapshots captured every N frames into a memory-bounded ring; older snapshots evict when the cap is reached."
      >
        <SettingRow
          label="Enable rewind"
          inherited={null}
          overridden={false}
          toggle={{
            checked: props.settings.rewindEnabled(),
            onChange: (v) => props.settings.setRewindEnabled(v),
          }}
          description="Captures save-state snapshots while playing."
        />
        <SettingRow
          label="Capture interval"
          inherited={null}
          overridden={false}
          disabled={!props.settings.rewindEnabled()}
          select={{
            value: String(props.settings.rewindCaptureIntervalFrames()),
            options: intervalOptions,
            onChange: (v) => props.settings.setRewindCaptureIntervalFrames(Number(v)),
          }}
          description="Lower = smoother rewind, more CPU + RAM. Higher = coarser rewind, cheaper."
        />
        <SettingRow
          label="Buffer cap"
          inherited={null}
          overridden={false}
          disabled={!props.settings.rewindEnabled()}
          select={{
            value: String(props.settings.rewindBufferMegabytes()),
            options: bufferOptions,
            onChange: (v) => props.settings.setRewindBufferMegabytes(Number(v)),
          }}
          description="Hard cap on RAM. Cores with large save states (SNES ~300 KB / snap) get fewer seconds of history per MB."
        />
      </SettingsCard>
    </div>
  );
};

// --- Shaders -----------------------------------------------------------

export const ShadersSettings: Component<{ settings: SettingsStore }> = (props) => {
  const presetOptions = createMemo(() => [
    { value: "system-default", label: shaderPresetLabel("system-default") },
    ...shaderPresets().map((p) => ({ value: p.name, label: p.displayName })),
  ]);

  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="Preset + bloom">
        <SettingRow
          label="Shader preset"
          inherited={null}
          overridden={false}
          select={{
            value: props.settings.shaderPreset(),
            options: presetOptions(),
            onChange: (v) => props.settings.setShaderPreset(v),
          }}
          description="Applies during the final blit. Per-system + per-game overrides win at launch."
        />
        <SettingRow
          label="Phosphor bloom amount"
          inherited={null}
          overridden={false}
          slider={{
            min: 0,
            max: 1,
            step: 0.05,
            value: props.settings.bloomAmount(),
            onInput: (v) => props.settings.setBloomAmount(v),
          }}
          description="OA-wide source/blur mix. Only meaningful when the active preset's base is Phosphor. Per-system + per-game overrides take precedence."
        />
      </SettingsCard>
    </div>
  );
};

// --- Profile -----------------------------------------------------------

/// A small set of presets so the operator doesn't have to remember
/// which emoji renders well at chip size. Custom emoji still works via
/// the freeform input.
const AVATAR_PRESETS = ["👤", "🎮", "👾", "🕹", "🤖", "🦊", "🐺", "⚡", "🌙", "🍄", "🦄", "🎯"];

export const ProfileSettings: Component<{ settings: SettingsStore }> = (props) => {
  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="Identity">
        <div class="flex items-center gap-4">
          <div class="grid h-16 w-16 shrink-0 place-items-center rounded-full border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 text-3xl">
            {props.settings.profileAvatar() || "👤"}
          </div>
          <div class="min-w-0 flex-1">
            <label class="block text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Display name
            </label>
            <input
              type="text"
              value={props.settings.profileDisplayName()}
              onInput={(e) => props.settings.setProfileDisplayName(e.currentTarget.value)}
              placeholder="Your name"
              maxLength={32}
              class="mt-1 w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-sm text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/60 focus:border-(--color-system-accent) focus:outline-none"
            />
          </div>
        </div>
      </SettingsCard>

      <SettingsCard title="Avatar">
        <div class="flex flex-col gap-3">
          <div class="flex flex-wrap gap-2">
            <For each={AVATAR_PRESETS}>
              {(preset) => {
                const isActive = () => props.settings.profileAvatar() === preset;
                return (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      props.settings.setProfileAvatar(preset);
                    }}
                    class="grid h-10 w-10 place-items-center rounded-full border text-xl transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                    classList={{
                      "border-(--color-system-accent) bg-(--color-system-accent)/15":
                        isActive(),
                      "border-white/10 bg-white/[0.04] hover:border-white/20": !isActive(),
                    }}
                    aria-pressed={isActive()}
                  >
                    {preset}
                  </button>
                );
              }}
            </For>
          </div>
          <label class="flex flex-col gap-1">
            <span class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Or paste a custom emoji
            </span>
            <input
              type="text"
              value={props.settings.profileAvatar()}
              onInput={(e) => {
                // Limit to a single grapheme — emoji can be multi-codepoint
                // but most chip-friendly avatars are single visible glyphs.
                const raw = e.currentTarget.value;
                const trimmed = [...raw][0] ?? "";
                props.settings.setProfileAvatar(trimmed);
              }}
              maxLength={8}
              class="w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-sm text-(--color-oa-ink) focus:border-(--color-system-accent) focus:outline-none"
            />
          </label>
          <p class="text-[0.65rem] text-(--color-oa-ink-dim)/70">
            The avatar drives the top-right profile chip on the
            Retroverse shell. Custom image uploads land in a future
            slice once the content-packs avatar pipeline ships.
          </p>
        </div>
      </SettingsCard>
    </div>
  );
};

// --- Cores -------------------------------------------------------------

export const CoresCategorySettings: Component = () => {
  // CoresPage embeds full-page chrome (header, etc.) — in the SETTINGS
  // tab context we let it own the whole center pane. The onBack
  // callback is a no-op because the SETTINGS sidebar handles
  // navigation away from the category. Wrapping in a thin div keeps
  // the page's height calc happy.
  return (
    <div class="h-full -mx-8 -my-6">
      <CoresPage onBack={() => { /* no-op: SETTINGS sidebar owns nav */ }} />
    </div>
  );
};

// --- Library / Media / BIOS — informational cards ---------------------

export const LibrarySettings: Component = () => (
  <div class="flex flex-col gap-4">
    <SettingsCard title="Library folders">
      <p class="text-[0.75rem] text-(--color-oa-ink-dim)">
        Library scanning, folder management, and the import wizard
        currently live in the legacy menu bar's{" "}
        <span class="text-(--color-system-accent)">Library → Library Manager…</span>{" "}
        surface. Flip Settings → Display → Experimental → Retroverse UI
        OFF to reach it, then back ON when you're done.
      </p>
      <p class="mt-3 text-[0.7rem] text-(--color-oa-ink-dim)/70">
        Wrapping the Library Manager body into this category lands in
        a follow-up slice — the legacy surface takes 5 store +
        callback props that need plumbing through RetroverseContext
        first.
      </p>
    </SettingsCard>
  </div>
);

export const MediaSettings: Component = () => (
  <div class="flex flex-col gap-4">
    <SettingsCard title="Per-platform media">
      <p class="text-[0.75rem] text-(--color-oa-ink-dim)">
        Per-system art slots (banner / clear-logo / console / controller
        / fanart / marquee / photo / wheel / background) are managed
        from the legacy menu bar's{" "}
        <span class="text-(--color-system-accent)">Library → Platform Media…</span>{" "}
        surface. The HOME tab's hero already reads from those slots
        immediately — files dropped under{" "}
        <code class="rounded border border-white/10 bg-black/40 px-1 font-mono text-[0.65rem]">
          {"<data_dir>/media/platform/<system_id>/"}
        </code>{" "}
        appear right away.
      </p>
      <p class="mt-3 text-[0.7rem] text-(--color-oa-ink-dim)/70">
        Wrapping the PlatformMediaDialog body into this tab is a
        follow-up; the existing component is modal-shaped and needs
        the variant="panel" lift treatment GameInfoModal got in
        Phase A Slice 3.
      </p>
    </SettingsCard>

    <SettingsCard title="Per-game art (libretro-thumbnails sync)">
      <p class="text-[0.75rem] text-(--color-oa-ink-dim)">
        Per-game cover art syncs from the libretro-thumbnails repo per
        system. Controlled from the menu bar's{" "}
        <span class="text-(--color-system-accent)">Tools → Sync media…</span>{" "}
        surface today.
      </p>
    </SettingsCard>
  </div>
);

export const BiosSettings: Component = () => {
  const [dataDir] = createResource(async () => {
    try {
      const mod = await import("../lib/dataDir");
      return await mod.getDataDir();
    } catch {
      return "";
    }
  });
  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="System directory">
        <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
          BIOS files live in:
        </p>
        <code class="mt-2 block break-all rounded border border-white/10 bg-black/40 px-2 py-1.5 font-mono text-[0.65rem] text-(--color-oa-ink)">
          {dataDir() ? `${dataDir()}/system/` : "Loading…"}
        </code>
        <p class="mt-3 text-[0.7rem] text-(--color-oa-ink-dim)">
          Drop the required BIOS files into this folder. OA verifies
          each at launch time and refuses to start a system with a
          missing / wrong BIOS (per-system sha-1 table). The pre-launch
          check surfaces the exact required filenames in the error
          toast if a BIOS is missing.
        </p>
      </SettingsCard>

      <SettingsCard title="Systems that need BIOS">
        <div class="space-y-2 text-[0.75rem] text-(--color-oa-ink-dim)">
          <p>
            <span class="text-(--color-oa-ink)">PCE-CD / TG-CD:</span>{" "}
            syscard3.pce
          </p>
          <p>
            <span class="text-(--color-oa-ink)">Sega CD:</span> bios_CD_E.bin /
            bios_CD_J.bin / bios_CD_U.bin
          </p>
          <p>
            <span class="text-(--color-oa-ink)">Saturn:</span> sega_101.bin /
            mpr-17933.bin
          </p>
          <p>
            <span class="text-(--color-oa-ink)">PSX:</span> SCPH7001.bin /
            SCPH7003.bin / SCPH1001.bin
          </p>
          <p>
            <span class="text-(--color-oa-ink)">Neo Geo CD:</span> neocd.bin /
            neocd_z.bin
          </p>
          <p>
            <span class="text-(--color-oa-ink)">3DO:</span> panafz1.bin /
            panafz10.bin
          </p>
          <p>
            <span class="text-(--color-oa-ink)">PC-FX:</span> pcfx.rom
          </p>
          <p>
            <span class="text-(--color-oa-ink)">Dreamcast:</span> dc_boot.bin
            + dc_flash.bin
          </p>
          <p>
            <span class="text-(--color-oa-ink)">PS2:</span> SCPH-70004_BIOS_V12_PAL_200.BIN
            (one of many — PCSX2 docs)
          </p>
          <p>
            <span class="text-(--color-oa-ink)">NDS:</span> bios7.bin / bios9.bin
            / firmware.bin
          </p>
          <p>
            <span class="text-(--color-oa-ink)">Atari Lynx / Jaguar:</span> lynxboot.img
            / jagboot.rom
          </p>
        </div>
        <p class="mt-3 text-[0.7rem] text-(--color-oa-ink-dim)/70">
          A live "is each BIOS present?" status grid lands in a
          follow-up — Rust gains a get_bios_status command that
          aggregates the existing per-system check_*_bios functions
          into one call.
        </p>
      </SettingsCard>
    </div>
  );
};

// --- About -------------------------------------------------------------

export const AboutSettings: Component = () => {
  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="Overlooked Arcade">
        <div class="space-y-3 text-sm">
          <p class="text-[0.65rem] uppercase tracking-[0.3em] text-(--color-system-accent)">
            Premium emulator frontend
          </p>
          <p class="text-(--color-oa-ink-dim)">
            A dedicated home for the consoles modern emulators forgot —
            TurboGrafx-16, Lynx, Atari 7800, SMS / Game Gear, MSX,
            ColecoVision, Vectrex, Virtual Boy, WonderSwan, and friends.
          </p>
          <p class="text-(--color-oa-ink-dim)">
            Non-commercial. A gift to the retro community. Built on
            forked C cores (Beetle PCE Fast, Mednafen, MAME modules)
            loaded as libretro .dlls. Shell is GPL-2.0; cores keep
            their upstream licenses.
          </p>
        </div>
      </SettingsCard>

      <SettingsCard title="Credits">
        <div class="space-y-2 text-[0.75rem] text-(--color-oa-ink-dim)">
          <p>
            <span class="text-(--color-oa-ink)">Cores:</span> Beetle PCE
            Fast / Mednafen / Stella / Mesen / nestopia / fceumm /
            snes9x / Genesis Plus GX / PicoDrive / Beetle Saturn /
            mupen64plus / Dolphin / Flycast / PCSX2 / DOSBox-pure /
            ScummVM and many more.
          </p>
          <p>
            <span class="text-(--color-oa-ink)">Shell:</span> Rust +
            Tauri 2 + wgpu (WGSL) + Solid + Tailwind. Libretro
            integration via libloading.
          </p>
          <p>
            <span class="text-(--color-oa-ink)">Art &amp; metadata:</span>{" "}
            libretro-thumbnails + LaunchBox + EmuMovies community packs.
          </p>
        </div>
      </SettingsCard>

      <SettingsCard title="Report a bug">
        <p class="text-[0.75rem] text-(--color-oa-ink-dim)">
          Found a crash or a bug? Open an issue on the project's
          GitHub. Include the contents of{" "}
          <span class="text-(--color-system-accent)">Help → Debug log…</span>{" "}
          (legacy menu bar) if the bug surfaced at runtime —
          frontend logs land in the same stream as Rust logs.
        </p>
      </SettingsCard>
    </div>
  );
};

// --- Storage -----------------------------------------------------------

type StorageSystemStatus = {
  cpuPercent: number;
  ramUsedBytes: number;
  ramTotalBytes: number;
  dataDirFreeBytes: number | null;
  dataDirTotalBytes: number | null;
};

function formatStorageBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(0)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  const gb = mb / 1024;
  if (gb < 1024) return `${gb.toFixed(1)} GB`;
  return `${(gb / 1024).toFixed(2)} TB`;
}

export const StorageSettings: Component = () => {
  const [dataDir] = createResource(async () => {
    try {
      const mod = await import("../lib/dataDir");
      return await mod.getDataDir();
    } catch {
      return "";
    }
  });
  const [storageInfo] = createResource(async () => {
    try {
      return await invoke<StorageSystemStatus>("get_system_status");
    } catch {
      return null;
    }
  });

  const isPortable = () => {
    const d = dataDir();
    if (!d) return false;
    // Portable mode places the data dir under <exe_dir>/settings/.
    // AppData mode places it under AppData/Roaming. A path-suffix
    // heuristic is good enough for the indicator.
    return /[\\\/]settings([\\\/]|$)/i.test(d) && !d.toLowerCase().includes("appdata");
  };

  const freePercent = () => {
    const info = storageInfo();
    if (!info || !info.dataDirFreeBytes || !info.dataDirTotalBytes) return null;
    return Math.round((info.dataDirFreeBytes / info.dataDirTotalBytes) * 100);
  };

  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="Data directory">
        <div class="space-y-2">
          <div class="flex items-baseline justify-between gap-3 text-[0.75rem]">
            <span class="shrink-0 text-(--color-oa-ink-dim)">Mode</span>
            <span class="text-right text-(--color-oa-ink)">
              {isPortable() ? "Portable" : "AppData"}
            </span>
          </div>
          <div class="flex flex-col gap-1">
            <span class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Path
            </span>
            <code class="break-all rounded border border-white/10 bg-black/40 px-2 py-1.5 font-mono text-[0.65rem] text-(--color-oa-ink)">
              {dataDir() ?? "Loading…"}
            </code>
          </div>
          <p class="text-[0.65rem] text-(--color-oa-ink-dim)/70">
            Portable mode is opted-in by dropping a `portable.txt`
            marker next to oa-shell.exe. Switching modes requires
            restarting OA.
          </p>
        </div>
      </SettingsCard>

      <SettingsCard title="Free space">
        <Show
          when={storageInfo() && storageInfo()!.dataDirFreeBytes !== null}
          fallback={
            <p class="text-[0.7rem] text-(--color-oa-ink-dim)/70">
              Couldn't match the data-dir drive against any sysinfo
              disk entry. Rare; happens on exotic mount setups.
            </p>
          }
        >
          <div class="space-y-2">
            <div class="flex items-baseline justify-between">
              <span class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                On data drive
              </span>
              <span class="text-2xl font-semibold text-(--color-oa-ink)">
                {formatStorageBytes(storageInfo()!.dataDirFreeBytes ?? 0)}
              </span>
            </div>
            <p class="text-[0.65rem] text-(--color-oa-ink-dim)">
              of {formatStorageBytes(storageInfo()!.dataDirTotalBytes ?? 0)}{" "}
              total ({freePercent()}% free)
            </p>
            <div class="h-1.5 w-full overflow-hidden rounded-full bg-white/5">
              <div
                class="h-full rounded-full bg-emerald-500"
                style={{ width: `${freePercent() ?? 0}%` }}
              />
            </div>
          </div>
        </Show>
      </SettingsCard>

      <SettingsCard title="Subdirectories">
        <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
          Save states / saves / logs / per-game overrides / scanned
          library / sync caches all live under the data directory.
        </p>
      </SettingsCard>
    </div>
  );
};

// --- Themes ------------------------------------------------------------

export const ThemesSettings: Component = () => {
  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="Default theme">
        <div class="space-y-3">
          <div class="flex items-center justify-between rounded-lg border border-(--color-system-accent)/40 bg-(--color-system-accent)/[0.08] px-4 py-3">
            <div class="min-w-0 flex-1">
              <p class="text-sm font-semibold text-(--color-oa-ink)">
                Retroverse
              </p>
              <p class="text-[0.65rem] text-(--color-oa-ink-dim)">
                The current top-toolbar IA — HOME / LIBRARY /
                COLLECTIONS / PLAY NOW / DISCOVER / SETTINGS.
                Experimental.
              </p>
            </div>
            <span class="rounded border border-emerald-400/40 bg-emerald-500/10 px-2 py-0.5 text-[0.55rem] uppercase tracking-widest text-emerald-300">
              Active
            </span>
          </div>
          <div class="flex items-center justify-between rounded-lg border border-white/10 bg-white/[0.02] px-4 py-3 opacity-50">
            <div class="min-w-0 flex-1">
              <p class="text-sm font-semibold text-(--color-oa-ink)">
                Legacy Shell
              </p>
              <p class="text-[0.65rem] text-(--color-oa-ink-dim)">
                Sidebar-driven layout. Available by toggling
                Settings → Experimental → Retroverse UI off.
              </p>
            </div>
            <span class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Switch via flag
            </span>
          </div>
        </div>
      </SettingsCard>

      <SettingsCard title="Future themes">
        <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
          Additional themes (Heroic-style / Kiosk cabinet) plus
          community theme packs land once the content-packs
          infrastructure ships (see Phase C6 in the rollout plan).
          Until then, this category lists the two built-in shells
          and lets the operator know to use the Experimental
          toggle to switch between them.
        </p>
      </SettingsCard>
    </div>
  );
};
