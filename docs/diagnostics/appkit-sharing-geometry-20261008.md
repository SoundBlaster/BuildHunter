# AppKit window-sharing geometry check — 2026-10-08

## Result and scope

A plain AppKit `NSWindow`, created through LLDB in the BuildHunter process,
reproduced the negative-size fault while Computer Use captured the window.
The control window contained no SwiftUI, Charts, toolbar, or BuildHunter controls.
The captured stack reached `NSThemeFrame._positionSharingIndicator`, and the
indicator reported `intrinsicContentSize = (-1, -1)`.

This identifies the source of the captured AppKit warning. It does not explain
the previously observed Charts `EXC_BREAKPOINT`, prove that every negative-size
warning has this source, or establish reproduction in a standalone application.
The process still hosted BuildHunter; only the control window was isolated.

## Environment

- Local `main`: `6b4e272c3399db57f6e301be1e5e05f9ffc8e1ab`.
- My Mac, Apple Silicon, macOS 27.0 (`26A428`).
- Xcode MCP launch with debugger attached; process PID `43888`.
- Breakpoint: `_os_log_fault_impl`.

## Procedure

1. Launch BuildHunter with Xcode MCP and attach LLDB.
2. Interrupt the process on the main thread and import AppKit in an Objective-C++
   expression. Create a plain window:

```objc
NSWindow *$controlWindow = [[NSWindow alloc]
    initWithContentRect:NSMakeRect(0, 0, 700, 450)
    styleMask:(NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
               NSWindowStyleMaskResizable)
    backing:NSBackingStoreBuffered defer:NO];
[$controlWindow setReleasedWhenClosed:NO];
[$controlWindow setTitle:@"AppKit Geometry Control"];
[$controlWindow center];
[$controlWindow makeKeyAndOrderFront:nil];
```

3. Set `breakpoint set -n _os_log_fault_impl`, then continue.
4. Select the control window in Computer Use and request a screenshot. The capture
   timed out while the breakpoint stopped the application's main thread.
5. Capture the backtrace and inspect the indicator's intrinsic size.
6. In a second capture, select frame 4 (`NSThemeFrame._positionSharingIndicator`)
   and evaluate `[[ (id)$x19 window] title]`. It returned
   `AppKit Geometry Control`, identifying the enclosing window.
7. Remove the diagnostic breakpoint, close the control window, and continue.

Register references and object addresses are specific to this arm64 capture;
they must be checked against the current stack/disassembly in another run.
No private API workaround was added to the application.

## Captured evidence

```text
_os_log_fault_impl
_NSViewValidateGeometry
NSViewValidateSize
-[NSView setFrameSize:]
-[NSThemeFrame _positionSharingIndicator]
-[NSWindowSharingSessionRecipientIndicator invalidateIntrinsicContentSize]
-[NSView _viewDidChangeAppearance:]
-[NSView _setSuperview:]
-[NSView addSubview:]
-[NSThemeFrame addTitlebarSubview:]
-[NSThemeFrame _updateButtons]
-[NSWindow _updateButtonsForWindowSharingSession]
-[NSWindow _setIsSelectivelyShared:]
___windowSelectiveSharingStateChangedNotification_block_invoke

(CGSize) $2 = (width = -1, height = -1)
```

[Original captured LLDB output](appkit-control-window-20261008.txt) includes the
addresses, offsets, breakpoint state, and PID. The indicator's own `window`
returned `nil` during insertion; the second capture identified the window via
its enclosing `NSThemeFrame`. The original `nil` output is retained explicitly.

The disassembly of `_positionSharingIndicator` showed two returned size values
saved in `v8`/`v9` immediately before its call to `setFrameSize:`. Both were `-1`.
The matching stack and size support an AppKit window-sharing layout issue in
this environment, rather than a size computed by the control window's content.

## Follow-up

Create a standalone AppKit reproducer and repeat selective window capture before
submitting Apple Feedback. Keep Charts crash diagnostics separate.
