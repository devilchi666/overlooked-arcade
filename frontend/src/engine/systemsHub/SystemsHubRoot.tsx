// Systems hub root — owns the in-pane navigation stack (Systems grid → system
// domain hub → domain editor), the breadcrumb, the Back integration, and
// per-level initial focus. Mounted by SettingsPanel under the "systems"
// category. Everything renders inside SettingsPanel's
// `data-nav-region="settings-content"` so the spatial engine drives it by
// geometry; the only nav wiring needed is the Back handler.

import {
  createEffect,
  createSignal,
  For,
  Match,
  on,
  onCleanup,
  Show,
  Switch,
  type Component,
} from "solid-js";
import { pushBackHandler } from "@oa/platform/nav";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import type { SettingsStore } from "@oa/platform/settings/store";
import { SystemsHub } from "./SystemsHub";
import { SystemHubDetail } from "./SystemHubDetail";
import { domainLabel, type DomainId } from "./domains";
import { DisplayVideoEditor } from "./domains/DisplayVideoEditor";
import { LayoutEditor } from "./domains/LayoutEditor";
import { CoreLauncherEditor } from "./domains/CoreLauncherEditor";
import { MediaEditor } from "./domains/MediaEditor";
import { PlatformMetadataEditor } from "./domains/PlatformMetadataEditor";
import { GameMetadataEditor } from "./domains/GameMetadataEditor";
import { InputEditor } from "./domains/InputEditor";
import { BiosEditor } from "./domains/BiosEditor";

type HubView =
  | { level: "grid" }
  | { level: "system"; systemId: SystemId }
  | { level: "domain"; systemId: SystemId; domain: DomainId };

export const SystemsHubRoot: Component<{ settings: SettingsStore }> = (props) => {
  const [stack, setStack] = createSignal<HubView[]>([{ level: "grid" }]);
  const view = (): HubView => stack()[stack().length - 1]!;
  const push = (v: HubView): void => {
    setStack((s) => [...s, v]);
  };
  const pop = (): void => {
    setStack((s) => (s.length > 1 ? s.slice(0, -1) : s));
  };
  const popTo = (depth: number): void => {
    setStack((s) => s.slice(0, Math.max(1, depth)));
  };

  // Drilled-in systems (level > 1) → the focused element on the current level
  // is null at this point; the editors read it from `view()`.
  const sysId = (): SystemId | null => {
    const v = view();
    return v.level === "grid" ? null : v.systemId;
  };
  const domain = (): DomainId | null => {
    const v = view();
    return v.level === "domain" ? v.domain : null;
  };

  // Back: while drilled in, register ONE back handler that pops a single level.
  // popBack() (B / Esc via the spatial engine) fires it BEFORE the takeover's
  // own onCancel, so Back steps up the hub instead of closing Settings. The
  // handler's lifetime is tied to "are we drilled in?" — balanced push/pop.
  createEffect(() => {
    if (stack().length <= 1) return;
    const dispose = pushBackHandler(() => pop());
    onCleanup(dispose);
  });

  // On every level change, focus the first interactive element of the new level
  // so the spatial ring lands somewhere sensible (the engine's focusin listener
  // syncs lastFocused). queueMicrotask lets the new content mount first.
  let contentRef: HTMLDivElement | undefined;
  createEffect(
    on(stack, () => {
      queueMicrotask(() => {
        contentRef
          ?.querySelector<HTMLElement>(
            "button:not([disabled]), a[href], input:not([disabled]), select:not([disabled])",
          )
          ?.focus();
      });
    }),
  );

  const crumbs = (): Array<{ label: string; depth: number }> => {
    const v = view();
    const out = [{ label: "Systems", depth: 1 }];
    if (v.level !== "grid") {
      out.push({ label: systemThemes[v.systemId]?.displayName ?? v.systemId, depth: 2 });
    }
    if (v.level === "domain") out.push({ label: domainLabel(v.domain), depth: 3 });
    return out;
  };

  return (
    <div class="flex h-full min-h-0 flex-col gap-4">
      {/* Breadcrumb — a plain div (NOT <nav>) so it shares the settings-content
          region and stays reachable by UP from the content. */}
      <div class="flex items-center gap-1 text-xs text-(--color-oa-ink-dim)" aria-label="Breadcrumb">
        <For each={crumbs()}>
          {(c, i) => (
            <>
              <Show when={i() > 0}>
                <span class="opacity-50">›</span>
              </Show>
              <button
                type="button"
                disabled={c.depth >= stack().length}
                onClick={(e) => {
                  e.currentTarget.blur();
                  popTo(c.depth);
                }}
                class="rounded px-1.5 py-0.5 transition hover:text-(--color-oa-ink) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent) disabled:font-semibold disabled:text-(--color-oa-ink)"
              >
                {c.label}
              </button>
            </>
          )}
        </For>
      </div>

      <div ref={(el) => (contentRef = el)} class="min-h-0 flex-1">
        <Switch>
          <Match when={view().level === "grid"}>
            <SystemsHub onPick={(id) => push({ level: "system", systemId: id })} />
          </Match>
          <Match when={view().level === "system"}>
            <SystemHubDetail
              systemId={sysId()!}
              onPickDomain={(d) => push({ level: "domain", systemId: sysId()!, domain: d })}
            />
          </Match>
          <Match when={view().level === "domain"}>
            <Switch
              fallback={
                <p class="text-sm text-(--color-oa-ink-dim)">This domain isn't wired yet.</p>
              }
            >
              <Match when={domain() === "display"}>
                <DisplayVideoEditor systemId={() => sysId()!} settings={props.settings} />
              </Match>
              <Match when={domain() === "layout"}>
                <LayoutEditor systemId={() => sysId()!} />
              </Match>
              <Match when={domain() === "core"}>
                <CoreLauncherEditor systemId={() => sysId()!} />
              </Match>
              <Match when={domain() === "media"}>
                <MediaEditor systemId={() => sysId()!} />
              </Match>
              <Match when={domain() === "platform-metadata"}>
                <PlatformMetadataEditor systemId={() => sysId()!} />
              </Match>
              <Match when={domain() === "game-metadata"}>
                <GameMetadataEditor systemId={() => sysId()!} />
              </Match>
              <Match when={domain() === "input"}>
                <InputEditor systemId={() => sysId()!} />
              </Match>
              <Match when={domain() === "bios"}>
                <BiosEditor systemId={() => sysId()!} />
              </Match>
            </Switch>
          </Match>
        </Switch>
      </div>
    </div>
  );
};

export default SystemsHubRoot;
