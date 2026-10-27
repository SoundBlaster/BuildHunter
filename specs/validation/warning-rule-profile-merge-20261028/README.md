# Warning actions and rule-profile integration — 2026-10-28

Runtime merge revision: `7ccc42d9b75b7ce686cddc9045a5eddb4b1e9019`. Main baseline: `c9040bb` and warning
actions head: `b74355b`. Both independent changes are preserved: optional exact
warning paths support Copy/Finder, and stopped rule profiles use a separate
bounded terminal handoff. A focused unit test exercises warning location and
late-event rejection alongside the final stopped profile. No owner acceptance,
new capability, new C ABI or target topology change is implied.

The Debug build and any executed tests are recorded separately in `checks.json`.
The macOS UI/Release jobs on the finished stack must still run in CI; prior
independent runs and local snapshots remain historical. Raw build outputs and
logs stay on verified TB5. Structural commands and actual warnings are recorded
in `structural-commands.json`; SpecPM unresolved capability warnings are not
represented as resolved.

The first build failed because the template integration test still matched the old
two-field warning event. The test was updated to ignore the new optional location;
the subsequent Debug build-for-testing passed. No local test body was executed in
this merge pass because the desktop was reserved; CI must execute the unit and UI
tests. Both build attempts and the exact commands are retained in checks.json.
