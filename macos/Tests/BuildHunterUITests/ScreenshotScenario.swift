import AppKit
import XCTest

/// The consumer supplies window queries and real navigation; never capture the desktop.
@MainActor
struct MacScreenshotScenario {
    let directory: URL

    static func run(prepare: (XCUIApplication) throws -> Void = { _ in },
                    _ body: (XCUIApplication, MacScreenshotScenario) throws -> Void) throws {
        let env = ProcessInfo.processInfo.environment
        guard env["SCREENER_TEST"] == "1" else { throw XCTSkip("Use the screenshot runner") }
        let app = XCUIApplication()
        let id = UUID().uuidString
        app.launchArguments = try JSONDecoder().decode([String].self, from: Data((env["SCREENER_APP_ARGUMENTS"] ?? "[]").utf8))
            + ["-AppleLanguages", "(\(try XCTUnwrap(env["SCREENER_LANGUAGE"])))",
               "-AppleLocale", try XCTUnwrap(env["SCREENER_LOCALE"])]
            + ["-AppleInterfaceStyle", env["SCREENER_APPEARANCE"] == "dark" ? "Dark" : "Light"]
        app.launchEnvironment = try JSONDecoder().decode([String: String].self, from: Data((env["SCREENER_APP_ENVIRONMENT"] ?? "{}").utf8))
        app.launchEnvironment.merge(["SCREENER_SCREENSHOTS": "1", "SCREENER_RUN_ID": id,
            "SCREENER_WINDOW_POINTS": try XCTUnwrap(env["SCREENER_WINDOW_POINTS"]),
            "SCREENER_APPEARANCE": env["SCREENER_APPEARANCE"] ?? "light"]) { _, value in value }
        app.launch()
        app.activate()
        defer { app.terminate() }
        try prepare(app)
        let probe = app.descendants(matching: .any).matching(identifier: "screener.capture.directory").firstMatch
        XCTAssertTrue(probe.waitForExistence(timeout: 20))
        let directory = URL(fileURLWithPath: try XCTUnwrap(probe.value as? String))
        guard directory.lastPathComponent == id else { throw CocoaError(.fileReadCorruptFile) }
        let capture = MacScreenshotScenario(directory: directory)
        let output = URL(fileURLWithPath: try XCTUnwrap(env["SCREENER_EXPORT_ROOT"])).appending(path: id)
        // Preserve partial captures even if the body or stop handshake fails.
        defer {
            do {
                try FileManager.default.createDirectory(at: output.deletingLastPathComponent(), withIntermediateDirectories: true)
                try FileManager.default.copyItem(at: directory, to: output)
            } catch { XCTFail("Could not export screenshot evidence: \(error)") }
        }
        try capture.waitForFile("ready.json")
        try body(app, capture)
        try capture.send("stop", presentation: "")
        try capture.waitForFile("stopped.json")
    }

    func checkpoint(_ name: String, presentation: String, window: XCUIElement) throws {
        XCTAssertTrue(window.exists && window.isHittable)
        try? FileManager.default.removeItem(at: directory.appending(path: "response.json"))
        try send(name, presentation: presentation)
        try waitForFile("response.json")
        let result = try JSONDecoder().decode([String: String].self, from: Data(contentsOf: directory.appending(path: "response.json")))
        guard result["name"] == name && result["state"] == "complete" else { throw CocoaError(.fileReadCorruptFile) }
        let system = directory.appending(path: "system")
        try FileManager.default.createDirectory(at: system, withIntermediateDirectories: true)
        try window.screenshot().pngRepresentation.write(to: system.appending(path: name + ".png"), options: .atomic)
    }
    private func send(_ name: String, presentation: String) throws {
        try JSONEncoder().encode(["name": name, "presentation": presentation]).write(to: directory.appending(path: "request.json"), options: .atomic)
    }
    private func waitForFile(_ name: String) throws {
        let deadline = Date().addingTimeInterval(20)
        while Date() < deadline {
            if FileManager.default.fileExists(atPath: directory.appending(path: name).path) { return }
            if let bytes = try? Data(contentsOf: directory.appending(path: "failure.json")) {
                XCTFail(String(decoding: bytes, as: UTF8.self))
                throw CocoaError(.fileReadCorruptFile)
            }
            Thread.sleep(forTimeInterval: 0.1)
        }
        XCTFail("Timed out waiting for \(name)")
        throw CocoaError(.fileReadNoSuchFile)
    }
}
