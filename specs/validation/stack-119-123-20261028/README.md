# Warning actions and folder chooser stack — 2026-10-28

Combined implementation/test revision: `d69a97a4429fa722ed5b19cd3b4e3d7d3cce2408`. Lower runtime source is
`7ccc42d9b75b7ce686cddc9045a5eddb4b1e9019`. The UI-only upper layer preserves
warning-action tests while adding explicit chooser focus. Earlier local Release
and UI-test observations in main-templates-20261027 are historical evidence for
`c9040bb` and original UI test revision `3d7ae11`; they are not proof for this tip.

Current/target map composition, runtime claims and acceptance remain unchanged
in the upper layer. Fresh source/map hashes and structural validation are recorded
here. The combined tip must pass applicable hosted macOS unit and UI tests before
merge; no local UI tests were run during the desktop coordination window.
