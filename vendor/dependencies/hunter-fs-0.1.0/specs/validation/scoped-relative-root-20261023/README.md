# Relative-root review correction

Hunter #26 review thread PRRT_kwDOVIWYR86vsR5W found that converting the stored
root to absolute made relative calls fail. The root now retains caller identity;
only the first open uses a separate absolute path. One regression checks `.` and
a relative nested root across metadata, listing, prepared measurement and markers.
All-features Debug/Release each pass 156 tests plus 2 doc tests on arm64 macOS;
clippy/rustfmt/HC/SpecPM pass (0 errors, 15 unresolved-capability warnings).
Fresh exact inputs and commands are recorded here. The earlier R3 receipts remain
historical for eafb6ff; no Windows descriptor implementation or live cloud proof.
