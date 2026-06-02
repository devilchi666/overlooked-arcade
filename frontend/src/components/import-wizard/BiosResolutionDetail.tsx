import { createSignal, For, Show, type Component, type JSX } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { open as pickFile } from "@tauri-apps/plugin-dialog";
import type { SystemId } from "../../themes/registry";
import { biosHintFor } from "./biosHints";

// Phase 1B Slice 5 — inline per-file BIOS detail.
//
// Renders inside SystemReadinessChecklist rows when the BIOS pill is
// ⚠ (missing / unknown-hash / error). Two affordances per file:
//   - "Pick BIOS file…" → opens @tauri-apps/plugin-dialog file picker,
//     calls install_bios_file Tauri command, which copies the chosen
//     file into <exe_dir>/system/ and reports back canonical-match vs
//     unknown-hash.
//   - Computed SHA-1 display when the file is present-but-mismatched
//     (so the operator can compare against their reference dump).
//
// No drag-drop — per docs/PARKING_LOT.md 2026-05-20 won't-fix; per-file
// picker is the operator-facing install affordance.

export type BiosFileStatus =
  | { kind: "ok"; sha1: string }
  | { kind: "unknownHash"; sha1: string }
  | { kind: "missing" }
  | { kind: "readError"; msg: string };

export type BiosFile = {
  expectedName: string;
  expectedSha1?: string;
  onDisk: BiosFileStatus;
  note?: string;
  optional?: boolean;
};

export type BiosResolutionDetailProps = {
  systemId: SystemId;
  files: BiosFile[];
  systemDir: string;
  /// Called after a successful install so the parent can refetch BIOS
  /// status + re-render. Parent owns the resource fetch lifecycle.
  onInstalled: () => void;
};

type InstallResult = {
  status: "ok" | "unknownHash";
  sha1: string;
  destination: string;
};

type FileBadgeStyle = { ring: string; bg: string; text: string; label: string };

function badgeFor(file: BiosFile): FileBadgeStyle {
  switch (file.onDisk.kind) {
    case "ok":
      return {
        ring: "border-emerald-400/40",
        bg: "bg-emerald-500/10",
        text: "text-emerald-300",
        label: "✓ Ready",
      };
    case "unknownHash":
      return {
        ring: "border-amber-400/40",
        bg: "bg-amber-500/10",
        text: "text-amber-300",
        label: "⚠ Unknown hash",
      };
    case "missing":
      return file.optional
        ? {
            ring: "border-white/15",
            bg: "bg-white/[0.04]",
            text: "text-(--color-oa-ink-dim)",
            label: "↪ Optional · not present",
          }
        : {
            ring: "border-amber-400/40",
            bg: "bg-amber-500/10",
            text: "text-amber-300",
            label: "⚠ Missing",
          };
    case "readError":
      return {
        ring: "border-red-400/40",
        bg: "bg-red-500/10",
        text: "text-red-300",
        label: "⚠ Read error",
      };
  }
}

const BiosResolutionDetail: Component<BiosResolutionDetailProps> = (props) => {
  // Per-file in-flight state. Keyed by expectedName; truthy = pick is
  // running (disable the button, show a spinner-like label).
  const [busy, setBusy] = createSignal<Record<string, boolean>>({});
  // Per-file install error text. Cleared on next pick attempt.
  const [errors, setErrors] = createSignal<Record<string, string>>({});

  async function pickAndInstall(file: BiosFile) {
    setErrors({ ...errors(), [file.expectedName]: "" });
    setBusy({ ...busy(), [file.expectedName]: true });
    try {
      const picked = await pickFile({
        directory: false,
        multiple: false,
        title: `Pick ${file.expectedName} for ${props.systemId}`,
        filters: [
          { name: "BIOS files", extensions: ["bin", "rom", "img", "pce", "zip"] },
          { name: "All files", extensions: ["*"] },
        ],
      });
      if (!picked || Array.isArray(picked)) {
        // Operator cancelled.
        return;
      }
      const result = await invoke<InstallResult>("install_bios_file", {
        sourcePath: picked,
        targetFilename: file.expectedName,
        systemId: props.systemId,
      });
      console.log(
        `[oa-bios] installed ${file.expectedName} → ${result.destination} (${result.status}, sha1 ${result.sha1})`,
      );
      props.onInstalled();
    } catch (e) {
      console.warn(`[oa-bios] install ${file.expectedName} failed:`, e);
      setErrors({ ...errors(), [file.expectedName]: String(e) });
    } finally {
      setBusy({ ...busy(), [file.expectedName]: false });
    }
  }

  function renderFileRow(file: BiosFile): JSX.Element {
    const badge = badgeFor(file);
    const needsPicker = file.onDisk.kind === "missing" || file.onDisk.kind === "unknownHash";
    const computedSha = (): string | undefined => {
      if (file.onDisk.kind === "ok" || file.onDisk.kind === "unknownHash") {
        return file.onDisk.sha1;
      }
      return undefined;
    };
    return (
      <div class="flex flex-col gap-1 rounded-md border border-white/5 bg-black/20 px-3 py-2 text-[0.7rem]">
        <div class="flex items-baseline gap-2">
          <code class="flex-1 truncate font-mono text-[0.75rem] text-(--color-oa-ink)">
            {file.expectedName}
          </code>
          <span
            class={`shrink-0 rounded border ${badge.ring} ${badge.bg} px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest ${badge.text}`}
          >
            {badge.label}
          </span>
        </div>
        <Show when={file.note}>
          <p class="text-[0.65rem] text-(--color-oa-ink-dim)">{file.note}</p>
        </Show>
        <Show when={file.expectedSha1 && file.onDisk.kind === "unknownHash"}>
          <div class="flex flex-col gap-0.5 text-[0.6rem] text-(--color-oa-ink-dim)">
            <span>
              Expected SHA-1:{" "}
              <code class="font-mono text-(--color-oa-ink)">{file.expectedSha1}</code>
            </span>
            <span>
              On disk SHA-1:{" "}
              <code class="font-mono text-amber-300">{computedSha()}</code>
            </span>
          </div>
        </Show>
        <Show when={file.onDisk.kind === "readError"}>
          <p class="text-[0.65rem] text-red-300">
            {file.onDisk.kind === "readError" ? file.onDisk.msg : ""}
          </p>
        </Show>
        <Show when={errors()[file.expectedName]}>
          <p class="text-[0.65rem] text-red-300">{errors()[file.expectedName]}</p>
        </Show>
        <Show when={needsPicker}>
          <div class="mt-1 flex items-center gap-2">
            <button
              type="button"
              class="rounded border border-(--color-system-accent)/40 bg-(--color-system-accent)/10 px-2 py-1 text-[0.65rem] uppercase tracking-widest text-(--color-system-accent-soft) hover:bg-(--color-system-accent)/20 disabled:cursor-not-allowed disabled:opacity-50"
              disabled={!!busy()[file.expectedName]}
              onClick={() => void pickAndInstall(file)}
            >
              {busy()[file.expectedName] ? "Installing…" : "Pick BIOS file…"}
            </button>
            <span class="text-[0.6rem] text-(--color-oa-ink-dim)">
              File copies to{" "}
              <code class="font-mono text-(--color-oa-ink-dim)">
                {props.systemDir}/{file.expectedName}
              </code>
            </span>
          </div>
        </Show>
      </div>
    );
  }

  return (
    <div class="flex flex-col gap-1.5">
      <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        BIOS file detail · pick a file from your collection to install it under the expected name
      </p>
      <For each={props.files}>{(f) => renderFileRow(f)}</For>
      <details class="mt-1 rounded-md border border-white/5 bg-black/20 px-3 py-2 text-[0.65rem] text-(--color-oa-ink-dim)">
        <summary class="cursor-pointer select-none text-[0.6rem] uppercase tracking-widest hover:text-(--color-oa-ink)">
          Where to get it
        </summary>
        <p class="mt-2 leading-relaxed">{biosHintFor(props.systemId)}</p>
      </details>
    </div>
  );
};

export default BiosResolutionDetail;
