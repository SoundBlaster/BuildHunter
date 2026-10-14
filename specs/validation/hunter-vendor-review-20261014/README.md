# Vendored snapshot inventory review

The new tests first failed: extra build.rs, src/hidden.rs, examples/hidden.rs and
a byte-identical symlinked lib.rs were accepted by the old verifier. All four
Python tests now pass; expected inventory is four pinned files plus UPSTREAM.json,
only src as an internal directory, with no links or special file types.
Provenance is checked as a regular non-link before parsing. Source hashes follow
inventory checks. No concurrent-mutation/remote-authenticity guarantee is claimed.
HC both maps/four profiles pass; SpecPM 0 errors, 17 unresolved capabilities.
Rust/core files and their constraints are unchanged; no redundant Rust tests or
benchmark rerun. Windows skips only the symlink-creation test due to host privilege
variation; extra file cases run on all CI platforms.
