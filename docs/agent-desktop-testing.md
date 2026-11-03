# Agent desktop testing

[`agent-desktop`](https://github.com/vercel-labs/agent-desktop) is an optional
external accessibility client for manual checks. It is not a BuildHunter
dependency and is not included in either Debug or Release. The app does not
start a server or expose a new automation endpoint for it. The tool is published
under Apache-2.0; consult its license and privacy documentation before use.

## Debug workflow

1. Build and launch the Debug configuration with the normal project instructions.
   Use the Debug-only Mock State menu for synthetic reports; never use a real scan
   or change a user's preferences merely to make an automation check pass.
2. Install a known version of `agent-desktop` outside the app. The upstream
   [installation instructions](https://github.com/vercel-labs/agent-desktop/blob/main/docs/install.md)
   describe supported packages and checksum verification. For example, in an
   isolated development environment: `npm install --prefix "$AGENT_DESKTOP_DIR" agent-desktop@1.1.0`.
   Add its `node_modules/.bin` to `PATH` for the commands below.
3. Run `agent-desktop skills get desktop`, `agent-desktop skills get platform`,
   `agent-desktop status`, and `agent-desktop permissions`. On macOS, the user
   must grant Accessibility access to the relevant client. Screen Recording is
   only needed for screenshots. Do not reset system privacy permissions.
4. Inspect the window, then act using a reference from the current snapshot:

   ```sh
   agent-desktop list-windows --app BuildHunter
   agent-desktop snapshot --app BuildHunter --window-id <window-id> -i --compact
   agent-desktop click '<snapshot-ref>'
   agent-desktop snapshot --app BuildHunter --window-id <window-id> -i --compact
   ```

   Substitute the actual window ID and ref. Re-snapshot after UI changes; refs
   belong to their snapshot. When multiple app windows exist, select the intended
   window explicitly. If Debug and TestFlight run together, use `list-apps` to
   identify their bundle IDs and target the appropriate app and window; do not
   assume the common display name uniquely identifies the Debug process.
   Check the resulting state: successful delivery of a click
   alone does not prove that the UI changed. Respect the tool's retry guidance.

Snapshots can include private paths and other UI data. Keep recordings, snapshots
and logs local unless their contents have been reviewed for sharing. This is a
supplement to, not a replacement for, the project's automated UI tests.

## Release assessment

The same external client can target a Release app: accessibility support belongs
in the app, while the automation client runs separately with user-granted macOS
Accessibility access. Apple's documentation says a sandboxed app can expose
accessibility, whereas an assistive app controlling other apps cannot itself be
sandboxed in that model. See Apple's [Accessibility Programming Guidelines](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Accessibility/cocoaAXIntro/cocoaAXintro.html)
and [App Sandbox restrictions](https://developer.apple.com/documentation/security/protecting-user-data-with-app-sandbox).
This does not establish App Review acceptance for a particular distribution.

An external Accessibility client can have broad control over the desktop; its
permission is not scoped to BuildHunter. A future purpose-built, user-enabled
agent interface may provide narrower operations and permissions; that is a
separate design from shipping this desktop controller inside the app.
