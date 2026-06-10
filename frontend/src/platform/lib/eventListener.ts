// Back-compat re-export. `listenScoped` moved to platform/api/eventsApi in the
// Phase 4.5 event corral (the one module allowed to touch @tauri-apps/api/event);
// this keeps the existing `@oa/platform/lib/eventListener` import path working.
// New code should import from `@oa/platform/api/eventsApi` directly and pass an
// `OA_EVENTS.*` channel.
export { listenScoped } from "@oa/platform/api/eventsApi";
