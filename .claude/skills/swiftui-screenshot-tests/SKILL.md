---
name: swiftui-screenshot-tests
description: Screenshot testing for the BuildHunter SwiftUI macOS app, driven by Python and runnable from Linux. Covers fetching CI test evidence, exporting screenshots, screen recordings and accessibility hierarchies from .xcresult bundles without Xcode, contact sheets, baseline pixel diffs with masks and triage, Screener .vtrace animation timelines, and rendering pure-geometry animation frames from a Linux Swift harness. Use it whenever a task involves UI screenshots, visual regressions, "what does the UI look like now", a failing XCUITest, CI artifacts like test.xcresult or ui-screenshots, animation or transition bugs, Screener traces, or comparing a PR's UI with main, even if the user never says "screenshot test". Also covers requests in Russian such as скриншот, скриншот-тесты, посмотри как выглядит UI, сравни скриншоты, and анимация.
---

# SwiftUI screenshot tests via Python

BuildHunter's UI tests run only on macOS CI runners, but every run uploads its
evidence. These scripts turn that evidence into something you can look at and
reason about from a Linux container: exported images, contact sheets, and
pixel diffs. Look at the images yourself with the Read tool. The point is to
see the UI, not to trust a number.

Dependencies: `pip install pillow zstandard`. `ffmpeg` is optional; it is
used to split screen recordings into frames. `gh` downloads CI artifacts.

Set two variables first. Every command below uses them, and absolute paths let
you run from anywhere:

```bash
SK="$(git rev-parse --show-toplevel)/.claude/skills/swiftui-screenshot-tests/scripts"
SCRATCH=<session scratchpad>/screens   # outputs never go in the repo
```

CI artifacts are downloaded data, so run the scripts with `python3 -I`.

| Evidence | Where it comes from | Tool |
|---|---|---|
| UI test screenshots | `ui-screenshots/` in the `macos-test-evidence` artifact | `compare_screenshots.py`, `contact_sheet.py` |
| Failure screenshots, screen recordings, hierarchy dumps | `test.xcresult` in the same artifact | `xcresult_attachments.py` |
| Animation frame timelines | Screener `.vtrace` bundles (`screener-traces/`, Debug builds with `BUILDHUNTER_SCREENER=1`) | `vtrace.py` |
| Animation geometry without a Mac | a Linux SwiftPM harness over the pure `Domain/` code | `render_sunburst_frames.py`, [references/linux-harness.md](references/linux-harness.md) |

## 1. Get the evidence

For a PR, take its head branch and base SHA from the REST API. `gh pr view`
uses GraphQL, which this environment blocks:

```bash
gh api repos/SoundBlaster/BuildHunter/pulls/37 --jq '.head.ref, .head.sha, .base.ref, .base.sha'
gh api repos/SoundBlaster/BuildHunter/pulls/37/files --jq '.[].filename'   # what the PR touches
```

```bash
$SK/fetch_ci_evidence.sh "$SCRATCH/ci-pr" --branch <head.ref>      # newest finished macos.yml run
$SK/fetch_ci_evidence.sh "$SCRATCH/ci-base" --commit <base.sha>   # baseline: the PR's own base
$SK/fetch_ci_evidence.sh "$SCRATCH/ci-run" --run 37542135009       # one exact run
```

- Use a fresh directory per run, so screenshots from different runs never mix.
- The script prints the run's conclusion, branch and full head SHA. Check that
  the SHA is the commit you mean before drawing conclusions.
- Prefer the run at the PR's base SHA as the baseline over "latest main".
  Otherwise unrelated changes merged since then show up as diffs.
- Cancelled runs have no evidence, so the script skips them.
- A run still in progress has no artifact yet.
- Artifacts expire after 7 days. Older runs need a re-run, or a local macOS
  run of `scripts/ci/macos.sh test`.

## 2. Look at a run

```bash
python3 -I $SK/contact_sheet.py "$SCRATCH/ci-pr/ui-screenshots" --out "$SCRATCH/pr.png" --title "PR 37 UI"
```

`ui-screenshots/` has UUID file names. For readable tiles, export through the
xcresult instead. That groups images by test and names them after their
attachment:

```bash
python3 -I $SK/xcresult_attachments.py "$SCRATCH/ci-pr/test.xcresult" --list
python3 -I $SK/xcresult_attachments.py "$SCRATCH/ci-pr/test.xcresult" --out "$SCRATCH/pr-shots" --type png
python3 -I $SK/contact_sheet.py "$SCRATCH/pr-shots" --out "$SCRATCH/pr.png"
```

Read the sheet, then open individual images at full size when a detail matters.
Tiles are downscaled and small text is unreadable on a sheet.

## 3. Diagnose a failing UI test

```bash
python3 -I $SK/xcresult_attachments.py test.xcresult --failures-only --list
python3 -I $SK/xcresult_attachments.py test.xcresult --out "$SCRATCH/fail" --test testName --video-fps 2
```

A failed run's evidence has only `test.xcresult` and `test.log`: the CI script
stops at the failing `xcodebuild`, before `xcresulttool` writes
`ui-screenshots/`, `test-summary.json` and the bundle's `database.sqlite3`.
The script then reads the bundle's object graph instead, so the same commands
work. Grep `test.log` for `error:` to find the failing assertion first.

This exports everything the failing test attached. That covers screenshots
taken at failure, `kXCTAttachmentScreenRecording` MP4s split into
`*_frames/frame_NNNN.png`, and text attachments such as the accessibility
hierarchy (`app.debugDescription`). Read the hierarchy text for the element's
real identifier, value and frame. XCTest also attaches a
`Debug description for <query>` text when a wait fails, which shows what the
element really reported. Compare it with the video frame: text that changed on
screen while the element's value stayed old is an accessibility refresh bug,
not a navigation bug. Then step through the video frames to see
what was on screen when the assertion fired. Typical root causes found this
way:
- a click landed on a label instead of the control;
- a popover inherited the wrong environment;
- an element existed but was off-screen.

`manifest.json` in the export records the test, result, activity title,
failure association and timestamp for each file.

## 4. Visual regression: compare against a baseline

The baseline is usually the same tests on `main`, from the same runner image.

```bash
python3 -I $SK/compare_screenshots.py "$SCRATCH/ci-main/ui-screenshots" "$SCRATCH/ci-pr/ui-screenshots" \
  --diff-dir "$SCRATCH/diff" --ignore 0,0,1024,24 --ignore 0,700,1024,68
```

- Images are paired by test and attachment name, so either side can be a raw
  `ui-screenshots/` folder, an `xcresult_attachments.py` export, or a single
  file. A name a test attaches several times is compared per occurrence
  (`name`, `name#2`, …, in capture order).
- A pixel counts as changed when any channel moves more than
  `--pixel-threshold` (default 16).
- An image fails when more than `--tolerance` of its pixels change (default
  0.1%), or when its size changes.
- Every image with any changed pixel gets a `baseline | current | diff`
  triptych: changed pixels are red on a faded baseline, inside an orange
  bounding box. File names start with the status (`fail__`, `size__`, or
  `pass__` for changes under tolerance), so failures sort first.
- `report.md` and `report.json` summarize the run.
- The exit code is 1 if anything failed or went missing.

CI screenshots come from `app.screenshot()`, which captures the whole
1024×768 VM screen. So the two `--ignore` rectangles above are almost always
needed: the menu bar (with its clock) and the Dock. Without them every image
"fails" by about 0.12%.

### Triage every flagged image

Read each triptych and put it in one bucket. A number alone never tells you
which.

1. **Intended change.** The PR meant to change this view. Example: the scan
   profile popover after a chart redesign. Say so, and describe what changed.
2. **Noise.** Content that varies run to run. Mask it with `--ignore`, or fix
   the test so it is deterministic. Known noise in this repo:
   - temp-folder names with UUIDs (`BuildHunter UI <UUID>`) in titles, paths
     and the path field (`testFullFolderPathCanBeCopiedAfterOpeningARealTarget`);
   - system UI whose content varies run to run, even on the same OS build. For
     example, the NSOpenPanel sidebar sometimes gains "Recents"/"Shared" rows
     (`testOpenFolderPresentsAndDismissesPicker`);
   - the menu bar clock and Dock badges;
   - a blinking text caret (about 2×16 px in "Filter folders") and
     anti-aliasing on toggle edges. These stay under tolerance and show up as
     `pass__` triptychs.
3. **Regression.** An unrelated view changed: shifted layout, clipped text, a
   wrong color, a missing element. Report it with the triptych path and the
   bounding box.

When unsure whether a change is intended, read the PR's patch for the files
that render that view before calling it a regression. Use
`gh api repos/SoundBlaster/BuildHunter/pulls/<n>/files --jq '.[] | .filename, .patch'`.
A change in a view the PR never touches is the strongest regression signal.

## 5. Animations: Screener traces

A screenshot catches one instant. Transitions such as the sunburst zoom need a
timeline. The Screener pilot (the DaisyDisk navigation PR, SoundBlaster/BuildHunter#38)
adds `DiagramTraceRecorder`. With it, Debug builds launched with
`BUILDHUNTER_SCREENER=1` record a `.vtrace`, and CI copies traces into
`screener-traces/` in the evidence artifact. A trace holds markers like
`navigation.begin`, `navigation.phase` and `chart.layout`, plus window frames
every ~40 ms.

First check that the run has traces:
`ls "$SCRATCH/ci-pr/screener-traces"`. Branches without the pilot have no
traces, and neither does `main` until #38 merges. Without traces, use
section 6 for the geometry, or section 3's screen-recording frames when a test
failed. A trace with markers but no frames, or with `navigation.skipped`,
means the animation never ran (for example, Reduce Motion on the runner).
Report that finding instead of judging the animation.

```bash
python3 -I $SK/vtrace.py list "$SCRATCH/ci-pr"                    # every .vtrace under a folder, newest first
python3 -I $SK/vtrace.py timeline TRACE.vtrace --kinds marker     # what happened, in ms since start
python3 -I $SK/vtrace.py sheet TRACE.vtrace --after navigation.begin --window-ms 800 --out "$SCRATCH/nav.png"
```

`sheet` labels each frame with the latest marker and its offset ("navigation.begin
+120ms"), so a sheet reads as a flip-book of the transition. Check that the
sequence is continuous: no frame where the selection jumps, the ring count
changes abruptly, or a sector appears from nowhere. Frames are captured with
`cacheDisplay` of the window's content view, which does capture in-flight
SwiftUI animation. Frames are RGBA, and most of each frame is transparent,
because the window background belongs to the window frame, not the content
view. The scripts composite them onto white (`--background ececec` gives a
light window color). A viewer that drops alpha shows a black window with
invisible black text; that is the viewer, not the app. If consecutive frames
are identical while the markers say an animation is running, that is a
capture limitation, not proof the UI froze.

The `.vtrace` format: `manifest.json`; `timeline.jsonl`, one record per line
with `sequence`, `monotonicNanoseconds`, `kind`
(sessionStarted/marker/keyframe/thumbnail/sessionEnded), `name`, `metadata`
and `blob`; and `frames/`. The reader tolerates a torn last line from a trace
still being written.

## 6. Animation geometry from Linux

When the question is "is the trajectory right?" rather than "does AppKit draw
it right?", skip the Mac entirely. Compile the pure geometry code in
`macos/Sources/BuildHunter/Domain/` with a Linux Swift toolchain. A small
executable dumps frames as JSON, and you render them:

```bash
swift run --package-path "$SCRATCH/harness" Dump descend > "$SCRATCH/descend.json"
python3 -I $SK/render_sunburst_frames.py "$SCRATCH/descend.json" --out "$SCRATCH/descend.png" --center-radius 0.21
```

This makes iterating on an animation take seconds instead of a CI round trip.
It also lets the same Swift Testing suites run on Linux. See
[references/linux-harness.md](references/linux-harness.md) for the toolchain
install, package layout and JSON format.

## Writing UI tests that screenshot well

When you add or change a UI test in `macos/Tests/BuildHunterUITests/`:
- Name attachments with stable, descriptive names, e.g.
  `attachScreenshot(named: "diagram-folder-hover", from: app)`. Comparisons
  pair images by test and name, so renaming one turns it into "missing" plus
  "new".
- Keep `lifetime = .keepAlways`. Otherwise passing tests drop their
  attachments.
- Drive views with deterministic data. Use the toolbar mock-state menu
  (`selectMockState("results", in:app:)`) and an isolated
  `BUILDHUNTER_SETTINGS_SUITE` instead of real folders. Where a real folder is
  unavoidable, expect its UUID path in the image.
- Attach the hierarchy (`XCTAttachment(string: app.debugDescription)`) next to
  any screenshot taken on a failure path. It is the cheapest way to diagnose
  the failure from Linux.
- `scripts/ci/macos.sh` fails when fewer than 14 PNG attachments are exported.
  Raise that floor when you add screenshots, so a silently skipped test is
  noticed.

## Reporting back

Lead with the verdict: "no visual changes outside the scan profile popover",
or "regression in X". Then give a short table of flagged images and their
buckets. Send the contact sheet and the relevant triptychs to the user as
files. They want to see the pictures, not just read about them. State which
two runs (SHAs) you compared.
