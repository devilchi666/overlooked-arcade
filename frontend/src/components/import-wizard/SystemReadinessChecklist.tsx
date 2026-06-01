import {
  createMemo,
  createResource,
  For,
  Show,
  type Accessor,
  type Component,
  type JSX,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { systemThemes, type SystemId } from "../../themes/registry";

// Phase 1B Slice 3 — Per-system readiness checklist.
//
// Mounts in two surfaces with the same shape:
//   1) New wizard Step 3 — sourced from commitRowsToEntries()
//      systemIds, surfaced between the per-ROM table (Step 2) and
//      Confirm (Step 4).
//   2) Settings → Library → "System readiness" card — sourced from
//      the operator's shipped library (library.state.entries
//      grouped by systemId).
//
// Five pills per system row:
//   - Core installed     (real status via list_cores)
//   - BIOS present       (real status via get_bios_status)
//   - Default bindings   (always ✓ — every onboarded system has them)
//   - Core options       (placeholder — coming Slice 4)
//   - Per-game overrides (placeholder — coming Slice 4)
//
// Resolution actions:
//   - ⚠ BIOS → "Open BIOS folder" (working, calls open_bios_folder).
//   - ⚠ Core → "Install core…" stub (toast pointing at Slice 4).

type CoreEntry = {
  fileName: string;
  libraryName: string;
  libraryVersion: string;
  validExtensions: string;
};

type BiosEntryStatus = "ok" | "unknownHash" | "missing" | "error";

type BiosStatusEntry = {
  slug: string;
  label: string;
  required: string;
  status: BiosEntryStatus;
  detail: string;
};

type BiosStatusResponse = {
  systemDir: string;
  entries: BiosStatusEntry[];
};

export type SystemReadinessChecklistProps = {
  /// Systems to render. Wizard surface = unique systemIds from
  /// commitRowsToEntries(); Settings surface = unique systemIds
  /// from library.state.entries.
  systems: Accessor<SystemId[]>;
  /// Override the empty-state copy. Wizard says "No systems in
  /// this scan"; Settings says "Library is empty — import a folder
  /// to get started."
  emptyStateLabel?: string;
};

type PillState = "ready" | "warning" | "na" | "coming";

const PILL_STYLES: Record<PillState, { ring: string; bg: string; text: string }> = {
  ready: {
    ring: "border-emerald-400/40",
    bg: "bg-emerald-500/10",
    text: "text-emerald-300",
  },
  warning: {
    ring: "border-amber-400/40",
    bg: "bg-amber-500/10",
    text: "text-amber-300",
  },
  na: {
    ring: "border-white/15",
    bg: "bg-white/[0.04]",
    text: "text-(--color-oa-ink-dim)",
  },
  coming: {
    ring: "border-white/10",
    bg: "bg-white/[0.02]",
    text: "text-(--color-oa-ink-dim)",
  },
};

function Pill(props: {
  state: PillState;
  label: string;
  detail?: string;
}): JSX.Element {
  const s = PILL_STYLES[props.state];
  return (
    <div class={`flex flex-col gap-0.5 rounded border ${s.ring} ${s.bg} px-2 py-1`}>
      <span class={`text-[0.6rem] font-semibold uppercase tracking-widest ${s.text}`}>
        {props.label}
      </span>
      <Show when={props.detail}>
        <span class="text-[0.6rem] text-(--color-oa-ink-dim)" title={props.detail}>
          {props.detail!.length > 40 ? `${props.detail!.slice(0, 38)}…` : props.detail}
        </span>
      </Show>
    </div>
  );
}

/// Cross-reference installed cores' valid_extensions against the system's
/// registered extensions. Returns true if ANY installed core handles AT
/// LEAST ONE of the system's extensions. Matches the loose semantics the
/// rest of OA uses ("can this system launch at all?") — a stricter "this
/// system has its preferred default core installed" check needs the
/// per-system default-core registry which lives in cores.json at runtime
/// (deferred to a polish pass).
function coreInstalledFor(systemId: SystemId, cores: CoreEntry[] | undefined): boolean {
  if (!cores) return false;
  const sysExts = new Set(
    systemThemes[systemId]?.extensions.map((e) => e.toLowerCase()) ?? [],
  );
  if (sysExts.size === 0) return false;
  for (const c of cores) {
    for (const raw of (c.validExtensions ?? "").split("|")) {
      const ext = raw.trim().toLowerCase().replace(/^\./, "");
      if (ext && sysExts.has(ext)) return true;
    }
  }
  return false;
}

const SystemReadinessChecklist: Component<SystemReadinessChecklistProps> = (props) => {
  const [cores] = createResource(async () => {
    try {
      return await invoke<CoreEntry[]>("list_cores");
    } catch (e) {
      console.warn("[oa-readiness] list_cores failed:", e);
      return [] as CoreEntry[];
    }
  });

  const [bios] = createResource(async () => {
    try {
      return await invoke<BiosStatusResponse>("get_bios_status");
    } catch (e) {
      console.warn("[oa-readiness] get_bios_status failed:", e);
      return { systemDir: "", entries: [] as BiosStatusEntry[] };
    }
  });

  // Pre-compute the BIOS lookup-by-slug for fast per-row queries.
  const biosBySlug = createMemo(() => {
    const m = new Map<string, BiosStatusEntry>();
    for (const e of bios()?.entries ?? []) {
      m.set(e.slug, e);
    }
    return m;
  });

  async function openBiosFolder() {
    try {
      await invoke("open_bios_folder");
    } catch (e) {
      console.warn("[oa-readiness] open_bios_folder failed:", e);
    }
  }

  function emitToast(text: string) {
    // The shell-wide toast surface listens on the same event the
    // SET_MESSAGE libretro env-callback emits (see Phase D batch).
    // No frontend toast helper exists yet — fall back to a console
    // log so the operator sees SOMETHING via the dev devtools while
    // we wait for the proper Slice 4 wire.
    console.log(`[oa-readiness] ${text}`);
    try {
      // Best-effort surface via window-level CustomEvent so any
      // future toast layer can subscribe. No-op today.
      window.dispatchEvent(
        new CustomEvent("oa://readiness-stub-toast", { detail: { text } }),
      );
    } catch {
      /* swallow */
    }
  }

  function renderRow(systemId: SystemId): JSX.Element {
    const theme = systemThemes[systemId];
    const displayName = theme?.displayName ?? systemId;

    // Core
    const hasCore = () => coreInstalledFor(systemId, cores());
    const corePill = (
      <Pill
        state={hasCore() ? "ready" : "warning"}
        label={hasCore() ? "✓ Core installed" : "⚠ No core"}
        detail={hasCore() ? undefined : "Drop a .dll into /cores/"}
      />
    );

    // BIOS
    const biosEntry = () => biosBySlug().get(systemId);
    const biosState = (): PillState => {
      const e = biosEntry();
      if (!e) return "na"; // system not in BIOS-requiring list → ↪ not required
      if (e.status === "ok") return "ready";
      return "warning";
    };
    const biosLabel = (): string => {
      const e = biosEntry();
      if (!e) return "↪ BIOS not required";
      if (e.status === "ok") return "✓ BIOS ready";
      if (e.status === "missing") return "⚠ BIOS missing";
      if (e.status === "unknownHash") return "⚠ Unknown hash";
      return "⚠ Read error";
    };
    const biosDetail = (): string | undefined => {
      const e = biosEntry();
      if (!e || e.status === "ok") return undefined;
      return e.detail || e.required;
    };
    const biosPill = <Pill state={biosState()} label={biosLabel()} detail={biosDetail()} />;

    // Bindings — always ✓ for any registered system.
    const bindingsPill = <Pill state="ready" label="✓ Bindings ready" />;

    // Core options — placeholder.
    const optionsPill = <Pill state="coming" label="— Core options" detail="Coming Slice 4" />;

    // KNOWN_GAME_BUGS overrides — placeholder.
    const overridesPill = <Pill state="coming" label="— Per-game overrides" detail="Coming Slice 4" />;

    return (
      <div
        data-system={systemId}
        class="flex flex-col gap-2 rounded-md border border-white/5 bg-white/[0.02] px-3 py-2.5 hover:border-white/15 hover:bg-white/[0.04]"
      >
        <div class="flex items-baseline justify-between">
          <h4 class="text-xs font-semibold text-(--color-oa-ink)">
            <span class="text-(--color-system-accent)">{theme?.shortName ?? systemId}</span>
            <span class="ml-2 text-(--color-oa-ink-dim)">{displayName}</span>
          </h4>
        </div>
        <div class="grid grid-cols-[repeat(auto-fit,minmax(160px,1fr))] gap-1.5">
          {corePill}
          {biosPill}
          {bindingsPill}
          {optionsPill}
          {overridesPill}
        </div>
        <Show when={!hasCore() || (biosEntry() && biosEntry()!.status !== "ok")}>
          <div class="mt-1 flex items-center gap-2 text-[0.7rem]">
            <Show when={!hasCore()}>
              <button
                type="button"
                class="rounded border border-amber-400/40 bg-amber-500/10 px-2 py-1 text-[0.65rem] uppercase tracking-widest text-amber-300 hover:brightness-110"
                onClick={() => emitToast(`Bulk core install for ${displayName} coming in Slice 4`)}
              >
                Install core…
              </button>
            </Show>
            <Show when={biosEntry() && biosEntry()!.status !== "ok"}>
              <button
                type="button"
                class="rounded border border-amber-400/40 bg-amber-500/10 px-2 py-1 text-[0.65rem] uppercase tracking-widest text-amber-300 hover:brightness-110"
                onClick={openBiosFolder}
              >
                Open BIOS folder
              </button>
            </Show>
          </div>
        </Show>
      </div>
    );
  }

  return (
    <div class="flex flex-col gap-3">
      <Show when={cores.loading || bios.loading}>
        <p class="rounded-md border border-white/5 bg-black/20 px-3 py-2 text-xs text-(--color-oa-ink-dim)">
          Loading readiness…
        </p>
      </Show>
      <Show
        when={!cores.loading && !bios.loading && props.systems().length === 0}
      >
        <p class="rounded-md border border-white/5 bg-black/20 px-3 py-2 text-xs text-(--color-oa-ink-dim)">
          {props.emptyStateLabel ?? "No systems in this scan."}
        </p>
      </Show>
      <Show when={!cores.loading && !bios.loading && props.systems().length > 0}>
        <div class="flex flex-col gap-2">
          <For each={props.systems()}>{(sid) => renderRow(sid)}</For>
        </div>
        <Show when={(bios()?.systemDir ?? "").length > 0}>
          <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            BIOS folder: <code class="font-mono text-(--color-oa-ink-dim)">{bios()?.systemDir}</code>
          </p>
        </Show>
      </Show>
    </div>
  );
};

export default SystemReadinessChecklist;
