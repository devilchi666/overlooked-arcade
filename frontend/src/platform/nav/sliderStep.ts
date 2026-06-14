// One slider step, clamped + snapped to the step grid. Shared by the legacy
// settings-row nav (settingsRowNav.tsx) and the spatial engine's slider
// adjust-mode (spatial.tsx). Pure — extracted here so the engine can reuse it
// without an import cycle (spatial → settingsRowNav → focus → spatial).

/// One slider step in `dir` (+1 = increase) from `value`, clamped to
/// [min, max] and snapped to the step grid relative to `min` so repeated
/// gamepad steps don't accumulate float drift (e.g. 0.05-step bloom). `min`
/// / `max` may be ±Infinity when the input declares no bound.
export function nextSliderValue(
  value: number,
  step: number,
  min: number,
  max: number,
  dir: 1 | -1,
): number {
  let next = value + dir * step;
  if (Number.isFinite(min)) next = Math.max(min, next);
  if (Number.isFinite(max)) next = Math.min(max, next);
  if (Number.isFinite(min) && step > 0) {
    next = min + Math.round((next - min) / step) * step;
    if (Number.isFinite(max)) next = Math.min(max, next);
    next = Math.max(min, next);
  }
  return next;
}
