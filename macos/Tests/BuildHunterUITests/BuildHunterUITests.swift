import XCTest

@MainActor
final class BuildHunterUITests: XCTestCase {
    func testMockStatesAndCaptureScreenshots() throws {
        let app = launchWindow()
        defer { app.terminate() }

        XCTAssertTrue(app.buttons["buildhunter.empty.openFolder"].waitForExistence(timeout: 10))
        attachScreenshot(named: "01-empty", from: app)

        // Re-enter empty after a populated report to cover the reset transition.
        let scenarios = ["results", "empty", "scanning", "stopped", "incomplete"]

        for (index, scenario) in scenarios.enumerated() {
            let mockStateMenu = app.descendants(matching: .any)
                .matching(identifier: "buildhunter.toolbar.mockState").firstMatch
            XCTAssertTrue(mockStateMenu.waitForExistence(timeout: 5))
            mockStateMenu.click()

            let optionID = "buildhunter.toolbar.mockState.scenario.\(scenario)"
            let option = app.descendants(matching: .any).matching(identifier: optionID).firstMatch
            XCTAssertTrue(option.waitForExistence(timeout: 5), "Missing mock-state option: \(optionID)")
            option.click()

            if scenario == "empty" {
                let openFolder = app.buttons["buildhunter.empty.openFolder"]
                XCTAssertTrue(openFolder.waitForExistence(timeout: 5),
                              "The empty scenario should return to folder selection")
                let reportTarget = app.descendants(matching: .any)
                    .matching(identifier: "buildhunter.report.target").firstMatch
                XCTAssertFalse(reportTarget.exists, "The empty scenario should clear the report")
            } else {
                let actionTitle = scenario == "scanning" ? "Stop" : "Rescan"
                XCTAssertTrue(app.buttons[actionTitle].waitForExistence(timeout: 5),
                              "The \(scenario) scenario should expose the \(actionTitle) action")
                if scenario == "incomplete" {
                    let warnings = app.descendants(matching: .any)
                        .matching(identifier: "buildhunter.report.warnings").firstMatch
                    XCTAssertTrue(warnings.waitForExistence(timeout: 5),
                                  "The incomplete scenario should expose warning details")
                }
            }
            attachScreenshot(named: String(format: "%02d-%@", index + 2, scenario), from: app)
        }
    }

    func testOpenFolderPresentsAndDismissesPicker() {
        let app = launchWindow()
        defer { app.terminate() }

        let openFolder = app.buttons["buildhunter.empty.openFolder"]
        XCTAssertTrue(openFolder.waitForExistence(timeout: 10))
        openFolder.click()

        let cancel = app.descendants(matching: .any)
            .matching(identifier: "CancelButton").firstMatch
        XCTAssertTrue(cancel.waitForExistence(timeout: 10), "Open Folder should present a folder picker")
        attachScreenshot(named: "folder-picker", from: app)
        cancel.click()

        XCTAssertTrue(openFolder.waitForExistence(timeout: 5), "Cancel should return to the empty window")
    }

    private func launchWindow() -> XCUIApplication {
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()

        // Relaunch can leave a macOS multiwindow app running without a window.
        if !app.windows.firstMatch.waitForExistence(timeout: 5) {
            app.typeKey("n", modifierFlags: .command)
        }
        XCTAssertTrue(app.windows.firstMatch.waitForExistence(timeout: 5),
                      "A BuildHunter window must exist before testing its controls")
        return app
    }

    private func attachScreenshot(named name: String, from app: XCUIApplication) {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
