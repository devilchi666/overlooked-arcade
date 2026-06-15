// Generic per-theme appearance renderer (Settings IA Slice 3). Reads the ACTIVE
// theme's declarative `settings_schema` and renders one SettingRow per control,
// bound to the theme's own per-theme storage (`useThemeSettings()`). The whole
// point: a theme — including a community one OA has never seen — declares its
// options in its manifest and they surface here automatically, with zero engine
// code per option. Mounted in Settings → Themes / Appearance below the picker.

import { For, Show, type Component } from "solid-js";
import { activeTheme } from "@oa/platform/theme/registry";
import { useThemeSettings } from "@oa/platform/theme/themeSettings";
import type { ThemeSettingControl } from "@oa/platform/theme/manifest";
import SettingRow from "@oa/platform/components/SettingRow";

const ControlRow: Component<{
  control: ThemeSettingControl;
  ts: ReturnType<typeof useThemeSettings>;
}> = (props) => {
  const c = props.control;
  const ts = props.ts;

  if (c.type === "toggle") {
    const value = (): boolean => ts.get<boolean>(c.key, c.default);
    return (
      <SettingRow
        label={c.label}
        hint={c.hint}
        inherited={null}
        overridden={value() !== c.default}
        onReset={() => ts.set(c.key, c.default)}
        toggle={{ checked: value(), onChange: (v) => ts.set(c.key, v) }}
      />
    );
  }

  if (c.type === "slider") {
    const value = (): number => ts.get<number>(c.key, c.default);
    const unit = c.unit;
    return (
      <SettingRow
        label={c.label}
        hint={c.hint}
        inherited={null}
        overridden={value() !== c.default}
        onReset={() => ts.set(c.key, c.default)}
        slider={{
          min: c.min,
          max: c.max,
          step: c.step,
          value: value(),
          format: unit ? (v) => `${v}${unit}` : undefined,
          onInput: (v) => ts.set(c.key, v),
        }}
      />
    );
  }

  // select
  const value = (): string => ts.get<string>(c.key, c.default);
  return (
    <SettingRow
      label={c.label}
      hint={c.hint}
      inherited={null}
      overridden={value() !== c.default}
      onReset={() => ts.set(c.key, c.default)}
      select={{
        value: value(),
        options: c.options.map((o) => ({ value: o.value, label: o.label })),
        onChange: (v) => ts.set(c.key, v),
      }}
    />
  );
};

export const AppearancePanel: Component = () => {
  const ts = useThemeSettings();
  const schema = (): ReadonlyArray<ThemeSettingControl> =>
    activeTheme()?.manifest.settings_schema ?? [];

  return (
    <Show
      when={schema().length > 0}
      fallback={
        <p class="text-sm text-(--color-oa-ink-dim)">
          This theme has no appearance options to configure.
        </p>
      }
    >
      <div class="flex flex-col gap-1">
        <For each={schema()}>{(c) => <ControlRow control={c} ts={ts} />}</For>
      </div>
    </Show>
  );
};

export default AppearancePanel;
