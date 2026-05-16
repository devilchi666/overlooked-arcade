# PATCHES — local modifications to vendored Beetle PCE Fast

Each entry is a numbered `.patch` file describing one local change against
upstream. Format: `NNNN-short-description.patch`.

Empty at vendor time. Future entries land here as we modify the core (e.g.,
strip libretro-glue references, expose internal state for save-states,
expose memory peeks for the inspector).
