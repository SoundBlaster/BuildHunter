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

    func testDiagramSharesReportAndReusesItsWindow() {
        let app = launchWindow()
        defer { app.terminate() }
        let scanWindow = app.windows.containing(.button, identifier: "buildhunter.toolbar.openDiagram").firstMatch
        XCTAssertTrue(scanWindow.waitForExistence(timeout: 5))
        selectMockState("results", in: scanWindow, app: app)
        scanWindow.buttons["buildhunter.toolbar.openDiagram"].click()

        let diagram = app.windows.containing(.staticText, identifier: "buildhunter.diagram.target").firstMatch
        XCTAssertTrue(diagram.waitForExistence(timeout: 10))
        let count = diagram.staticTexts["buildhunter.diagram.count"]
        XCTAssertTrue(count.waitForExistence(timeout: 5))
        expectValue("3 artifacts", of: count)
        let status = diagram.staticTexts["buildhunter.diagram.status"]
        expectValue("Scan complete", of: status)
        XCTAssertTrue(diagram.descendants(matching: .any)
            .matching(identifier: "buildhunter.diagram.chart").firstMatch.waitForExistence(timeout: 5))
        attachScreenshot(named: "diagram-results", from: app)

        // The value-based WindowGroup must reuse this report's companion window.
        activateWindow(titled: "BuildHunter", in: app)
        scanWindow.buttons["buildhunter.toolbar.openDiagram"].click()
        XCTAssertEqual(app.windows.count, 2)
        selectMockState("scanning", in: scanWindow, app: app)
        activateWindow(titled: "Demo Workspace — Artifact Diagram", in: app)
        expectValue("Scanning · live updates", of: status)
        XCTAssertTrue(diagram.staticTexts["Waiting for sizes"].waitForExistence(timeout: 5))
        attachScreenshot(named: "diagram-scanning", from: app)

        selectMockState("stopped", in: scanWindow, app: app)
        activateWindow(titled: "Demo Workspace — Artifact Diagram", in: app)
        expectValue("Scan stopped · partial results", of: status)
        XCTAssertTrue(diagram.descendants(matching: .any)
            .matching(identifier: "buildhunter.diagram.chart").firstMatch.waitForExistence(timeout: 5))
        attachScreenshot(named: "diagram-partial", from: app)

        selectMockState("scanning", in: scanWindow, app: app)
        activateWindow(titled: "Demo Workspace — Artifact Diagram", in: app)
        expectValue("Scanning · live updates", of: status)
        diagram.buttons[XCUIIdentifierCloseWindow].click()
        XCTAssertTrue(diagram.waitForNonExistence(timeout: 5))
        XCTAssertTrue(scanWindow.buttons["Stop"].exists,
                      "Closing the companion diagram must not stop its scan")
    }

    private func selectMockState(_ state: String, in window: XCUIElement, app: XCUIApplication) {
        activateWindow(titled: "BuildHunter", in: app)
        let menu = window.descendants(matching: .any)
            .matching(identifier: "buildhunter.toolbar.mockState").firstMatch
        XCTAssertTrue(menu.waitForExistence(timeout: 5))
        menu.click()
        let option = app.descendants(matching: .any)
            .matching(identifier: "buildhunter.toolbar.mockState.scenario.\(state)").firstMatch
        XCTAssertTrue(option.waitForExistence(timeout: 5))
        option.click()
    }

    private func activateWindow(titled title: String, in app: XCUIApplication) {
        // The companion window can cover the toolbar. Select the owner via the
        // native Window menu before interacting with its controls.
        app.menuBars.menuBarItems["Window"].click()
        let item = app.menuItems[title]
        XCTAssertTrue(item.waitForExistence(timeout: 5))
        item.click()
    }

    private func expectValue(_ text: String, of element: XCUIElement,
                             file: StaticString = #filePath, line: UInt = #line) {
        // SwiftUI Text exposes its content as AXValue on macOS, not AXTitle/label.
        let expectation = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value == %@", text),
                                                    object: element)
        XCTAssertEqual(XCTWaiter.wait(for: [expectation], timeout: 5), .completed,
                       "Expected static text value: \(text)", file: file, line: line)
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
