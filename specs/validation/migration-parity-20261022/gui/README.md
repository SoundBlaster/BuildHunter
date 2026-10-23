# GUI baseline integration — upper R2 layer

Adds three Swift Testing tests: the shared 28 GUI runs through the real
RustScanEventSource/Swift finite policy/Rust FFI, terminal-code vectors, and
invalid-UTF-8 lossy warning/failure handling. The JSON is a test-bundle resource;
both XcodeGen input and checked-in project are wired. No product policy changes.

The test source compiled and linked locally with Xcode 27.1 (27A9275), macOS 27.0
(26A428), arm64. Execution is **not locally passed**: the host stayed in dyld
`__open` before reaching main/test connection. Attempt 3 timed out with exit 65;
xcresult reports a launch failure, zero passed tests. Attempts 1/2 were interrupted
with exit 75. Attempt 1 revealed global custom Xcode locations; attempts 2/3
explicitly isolate OBJROOT/SYMROOT/SHARED_PRECOMPS_DIR on the verified TB5 checkout
route. Attempt 3 used normal Xcode ad-hoc signing instead of disabled signing.
No system security preferences or unrelated running Xcode project were changed.

`xcode-attempts.json` retains exact commands, log paths/hashes and result bundles
on TB5. `xcresult-summary.json` and `test-host-sample.txt` record the observed
blocker. This does not identify its underlying system cause. Hosted macOS unit
execution on the stack tip is required before claiming GUI parity or merging.

Final HC parse/profile checks, plist syntax and SpecPM validation are retained
separately here (0 errors / 39 existing warnings). Source hashes are current for
this layer; lower-layer receipts remain historical to their exact inputs.
This bridge integration suite does not test visible UI, sandbox signing/Store,
end-user cancellation latency, actual invalid-byte Finder actions or live iCloud.

Final source review corrected the new kind assertion: `ScanArtifact.scannerKind`
is only an optional raw bytecode/metadata override, not the ordinary GUI kind.
The witness now compares `artifact.kind` to the literal golden and checks the
optional override is nil for the folded GUI profile. No golden rows or product
code changed; the obsolete hosted attempt was cancelled before pushing this fix.

Attempt 4 (`build-for-testing`) compiles and links the final corrected source
with exit 0 (`TEST BUILD SUCCEEDED`). This compile-only receipt does not replace
the pending hosted suite execution.
