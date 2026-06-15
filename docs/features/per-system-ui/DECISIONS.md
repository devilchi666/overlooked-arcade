# Per-System Custom UI — Implementation Decisions

Implementation-level decisions made during the build. Separate from
plan §14 (those are strategic / cross-stage; these are concrete
"how do we wire this in code" calls).

## 2026-05-26 — SystemUIConfig storage: sibling file, not merged into SystemTheme

**Decision:** Per-system UI configs live in
`frontend/src/themes/systemUIConfigs.ts` (now
`frontend/src/platform/themes/systemUIConfigs.ts`), parallel to the existing
`frontend/src/themes/registry.ts` (now
`frontend/src/platform/themes/registry.ts`). Not merged into `SystemTheme`.

**Why:** Per plan §14.2, two options were viable:

- **Merge into `SystemTheme`** — one source of truth per system,
  cleaner long-term. But `registry.ts` already runs 897 lines × 40
  systems; layering 10-15 behavioral fields per entry bloats it
  significantly and forces every existing call site to think about
  the new fields.
- **Sibling file** — easier rollout (don't touch existing per-system
  color work), clearer separation while the Stage 1 shape is still
  shifting, easier rollback if the architecture changes during the
  build.

Operator picked sibling. Merging can happen later once Stage 1+2+3
settle the final shape — then `SystemTheme` extends with the locked
behavioral fields in one pass instead of in pieces.

**How to apply:** New per-system fields go in `systemUIConfigs.ts`,
keyed by `SystemId`. The `Record<SystemId, SystemUIConfig>` shape
forces every system to have an entry. Consumers import from
`themes/systemUIConfigs` directly; the existing `systemThemes` map
stays untouched.

**Revisit when:** Stage 3 ships (architecture stable enough to merge);
or if a consumer needs both visual + behavioral fields together
often enough that two imports feels awkward. *(Path note: both files now
live under `frontend/src/platform/themes/` after the `platform/` refactor.)*

---

## 2026-05-26 — `prefers-reduced-motion` lives in `frontend/src/lib/`

**Decision:** Created `frontend/src/lib/reducedMotion.ts` (now
`frontend/src/platform/lib/reducedMotion.ts`) as a
module-level Solid signal subscribed to a shared
`window.matchMedia("(prefers-reduced-motion: reduce)")` listener.
Consumers (boot animation framework, tile flourish system, transition
timing logic) import `prefersReducedMotion` directly.

**Why:** Cross-cutting OS preference shared by multiple consumers; one
subscription is cheaper than N. Module-level state is acceptable
because the value is global by definition (OS preference applies to
the whole app, not a component subtree).

**How to apply:** Consumers that need to short-circuit a long-form
animation read `prefersReducedMotion()` at the call site. The
canonical pattern is `transitionTiming() === "instant" ||
prefersReducedMotion()` → use the no-animation path; otherwise apply
the system's `transitionTiming` value.

**Revisit when:** Never expected to change — `prefers-reduced-motion`
is a stable OS-level CSS media query.
