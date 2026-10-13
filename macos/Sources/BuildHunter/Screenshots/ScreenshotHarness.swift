#if SCREENER_SCREENSHOTS
import AppKit
import SwiftUI
import ScreenerKit

/// One checkpoint session, explicitly registered presentation windows. No desktop capture.
@MainActor
final class MacScreenshotRecorder {
    static let shared = MacScreenshotRecorder()
    static let activated = ProcessInfo.processInfo.environment["SCREENER_SCREENSHOTS"] == "1"
        && UUID(uuidString: ProcessInfo.processInfo.environment["SCREENER_RUN_ID"] ?? "") != nil
    private struct Entry {
        weak var window: NSWindow?
        var ready: Bool
    }
    private var entries: [String: Entry] = [:]
    private var task: Task<Void, Never>?
    var directory: URL? {
        guard Self.activated, let id = ProcessInfo.processInfo.environment["SCREENER_RUN_ID"],
              let caches = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask).first else { return nil }
        return caches.appending(path: "Screener/Screenshots/\(id)", directoryHint: .isDirectory)
    }

    func register(_ window: NSWindow?, presentation: String, ready: Bool) {
        guard Self.activated else { return }
        entries[presentation] = Entry(window: window, ready: ready)
        if let window, let dimensions = ProcessInfo.processInfo.environment["SCREENER_WINDOW_POINTS"],
           let size = try? JSONDecoder().decode([Double].self, from: Data(dimensions.utf8)), size.count == 2 {
            let wanted = NSSize(width: size[0], height: size[1])
            if window.frame.size != wanted {
                var frame = window.frame
                frame.size = wanted
                window.setFrame(frame, display: true)
                window.center()
            }
            window.appearance = NSAppearance(named:
                ProcessInfo.processInfo.environment["SCREENER_APPEARANCE"] == "dark" ? .darkAqua : .aqua)
        }
        if task == nil, let directory, window != nil {
            task = Task { await record(in: directory) }
        }
    }

    private func record(in run: URL) async {
        let screener = Screener()
        do {
            try FileManager.default.createDirectory(at: run, withIntermediateDirectories: true)
            let trace = try await screener.startSession(name: "macOS screenshot checkpoints",
                appBundleID: Bundle.main.bundleIdentifier ?? "ScreenshotApp", tracesDirectory: run)
            try await screener.mark("screenshots.ready", metadata: ["backend": "AppKitCaptureSource", "orientation": "window"])
            try write(["state": "ready", "trace": trace.lastPathComponent], to: run.appending(path: "ready.json"))
            let started = ContinuousClock.now
            var count = 0
            let command = run.appending(path: "request.json")
            while seconds(since: started) < 180 {
                if let bytes = try? Data(contentsOf: command) {
                    let request = try JSONDecoder().decode(Request.self, from: bytes)
                    try FileManager.default.removeItem(at: command)
                    if request.name == "stop" {
                        try await screener.stopSession()
                        try write(["state": "stopped"], to: run.appending(path: "stopped.json"))
                        return
                    }
                    guard count < 16 else { throw CaptureError.checkpointLimit }
                    try await capture(request, screener: screener)
                    try write(["state": "complete", "name": request.name], to: run.appending(path: "response.json"))
                    count += 1
                }
                try await Task.sleep(for: .milliseconds(100))
            }
            throw CaptureError.sessionExpired
        } catch {
            try? write(["state": "failed", "error": String(describing: error)], to: run.appending(path: "failure.json"))
        }
        try? await screener.stopSession()
    }

    private func capture(_ request: Request, screener: Screener) async throws {
        let started = ContinuousClock.now
        var stableSince = ContinuousClock.now
        var previousFrame: NSRect?
        var previousScale: CGFloat?
        while seconds(since: started) < 12 {
            guard let entry = entries[request.presentation], let window = entry.window,
                  let frameView = window.contentView?.superview else {
                stableSince = .now
                try await Task.sleep(for: .milliseconds(100))
                continue
            }
            let scale = window.backingScaleFactor
            let eligible = entry.ready && window.isVisible && window.isKeyWindow && NSApp.isActive
            if !eligible || previousFrame != window.frame || previousScale != scale { stableSince = .now }
            previousFrame = window.frame
            previousScale = scale
            if eligible && seconds(since: stableSince) >= 0.8 {
                // The enclosing frame view includes the title bar. Validate rather than
                // assuming this public contentView relationship always spans the window.
                guard frameView.bounds.size == window.frame.size,
                      let screen = window.screen,
                      screen.visibleFrame.contains(window.frame) else { throw CaptureError.windowGeometry }
                let image = try AppKitCaptureSource(view: frameView, background: .window).capture()
                guard image.cgImage.width == Int((window.frame.width * scale).rounded()),
                      image.cgImage.height == Int((window.frame.height * scale).rounded()) else {
                    throw CaptureError.nonNativePixels
                }
                try await screener.mark("screenshots.checkpoint", metadata: [
                    "name": request.name, "presentation": request.presentation,
                    "pointWidth": String(Double(window.frame.width)), "pointHeight": String(Double(window.frame.height)),
                    "windowNumber": String(window.windowNumber), "title": window.title, "captureArea": "window-frame",
                    "capture.backend": "AppKitCaptureSource", "capture.bitDepth": String(image.cgImage.bitsPerComponent),
                    "capture.colorSpace": (image.cgImage.colorSpace?.name as String?) ?? "unknown"
                ])
                try await screener.recordFrame(image, reason: request.name)
                return
            }
            try await Task.sleep(for: .milliseconds(100))
        }
        throw CaptureError.presentationDidNotSettle
    }

    private func write(_ value: [String: String], to url: URL) throws {
        try JSONEncoder().encode(value).write(to: url, options: .atomic)
    }
    private func seconds(since start: ContinuousClock.Instant) -> Double {
        let value = start.duration(to: .now).components
        return Double(value.seconds) + Double(value.attoseconds) / 1e18
    }
    private struct Request: Decodable { let name: String; let presentation: String }
    private enum CaptureError: Error { case checkpointLimit, sessionExpired, windowGeometry, nonNativePixels, presentationDidNotSettle }
}

/// Transparent accessibility probe exposes the sandbox-resolved path to XCTest.
struct MacScreenshotProbe: NSViewRepresentable {
    let isReady: Bool
    let presentation: String
    func makeNSView(context: Context) -> ProbeView { ProbeView() }
    func updateNSView(_ view: ProbeView, context: Context) {
        view.ready = isReady
        view.presentation = presentation
        view.register()
    }
    static func dismantleNSView(_ view: ProbeView, coordinator: ()) {
        MacScreenshotRecorder.shared.register(nil, presentation: view.presentation, ready: false)
    }
    final class ProbeView: NSView {
        var ready = false
        var presentation = "main"
        override func viewDidMoveToWindow() { super.viewDidMoveToWindow(); register() }
        func register() {
            guard MacScreenshotRecorder.activated else { return }
            setAccessibilityElement(true)
            setAccessibilityRole(.staticText)
            setAccessibilityLabel("Screener screenshot capture")
            setAccessibilityIdentifier("screener.capture.directory")
            setAccessibilityValue(MacScreenshotRecorder.shared.directory?.path)
            MacScreenshotRecorder.shared.register(window, presentation: presentation, ready: ready)
        }
    }
}
#endif
