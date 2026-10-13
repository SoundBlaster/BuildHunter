#if SCREENER_SCREENSHOTS
import XCTest

@MainActor
final class BuildHunterScreenshotTests: XCTestCase {
    func testScreenshotBatch() throws {
        continueAfterFailure = false
        executionTimeAllowance = 240
        try MacScreenshotScenario.run(prepare: openMainWindowIfNeeded) { app, capture in
            let main = app.windows.containing(.button, identifier: "buildhunter.toolbar.openDiagram").firstMatch
            XCTAssertTrue(main.waitForExistence(timeout: 15))
            XCTAssertTrue(main.buttons["buildhunter.empty.openFolder"].exists)
            try capture.checkpoint("01-empty", presentation: "main", window: main)
            let menu = main.descendants(matching: .any).matching(identifier: "buildhunter.toolbar.mockState").firstMatch
            menu.click()
            let completed = app.descendants(matching: .any)
                .matching(identifier: "buildhunter.toolbar.mockState.scenario.results").firstMatch
            XCTAssertTrue(completed.waitForExistence(timeout: 5))
            completed.click()
            XCTAssertTrue(main.buttons["Rescan"].waitForExistence(timeout: 5))
            try capture.checkpoint("02-results", presentation: "main", window: main)
            main.buttons["buildhunter.toolbar.openDiagram"].click()
            let diagram = app.windows.containing(.staticText, identifier: "buildhunter.diagram.target").firstMatch
            XCTAssertTrue(diagram.waitForExistence(timeout: 10))
            XCTAssertTrue(diagram.descendants(matching: .any).matching(identifier: "buildhunter.diagram.chart").firstMatch.waitForExistence(timeout: 10))
            try capture.checkpoint("03-diagram", presentation: "diagram", window: diagram)
        }
    }

    func testRuntimeOptOut() throws {
        guard ProcessInfo.processInfo.environment["SCREENER_TEST"] == "1" else { throw XCTSkip("Use the screenshot runner") }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launchEnvironment = ["SCREENER_RUN_ID": UUID().uuidString,
                                 "BUILDHUNTER_SETTINGS_SUITE": "BuildHunter.Screenshots.OptOut"]
        app.launch()
        defer { app.terminate() }
        openMainWindowIfNeeded(app)
        XCTAssertTrue(app.buttons["buildhunter.empty.openFolder"].waitForExistence(timeout: 15))
        XCTAssertFalse(app.descendants(matching: .any).matching(identifier: "screener.capture.directory").firstMatch.exists)
    }

    private func openMainWindowIfNeeded(_ app: XCUIApplication) {
        let main = app.windows.containing(.button, identifier: "buildhunter.toolbar.openDiagram").firstMatch
        if !main.waitForExistence(timeout: 3) {
            app.menuBars.menuBarItems["File"].click()
            app.menuBars.menuItems["New BuildHunter Window"].click()
            XCTAssertTrue(main.waitForExistence(timeout: 10))
        }
    }
}
#endif
