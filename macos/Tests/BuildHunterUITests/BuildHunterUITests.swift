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
        let emptyStatus = diagram.staticTexts["buildhunter.diagram.emptyStatus"]
        XCTAssertTrue(emptyStatus.waitForExistence(timeout: 5))
        expectValue("Waiting for sizes", of: emptyStatus)
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

    func testDiagramHoverPreviewAndCenterNavigation() {
        let app = launchWindow()
        defer { app.terminate() }
        let scanWindow = app.windows.containing(.button, identifier: "buildhunter.toolbar.openDiagram").firstMatch
        selectMockState("results", in: scanWindow, app: app)
        scanWindow.buttons["buildhunter.toolbar.openDiagram"].click()
        let diagram = app.windows.containing(.staticText, identifier: "buildhunter.diagram.target").firstMatch
        XCTAssertTrue(diagram.waitForExistence(timeout: 10))
        let focus = diagram.staticTexts["buildhunter.diagram.focus"]
        expectValue("Demo Workspace", of: focus)
        let chart = diagram.descendants(matching: .any).matching(identifier: "buildhunter.diagram.chart").firstMatch
        XCTAssertTrue(chart.waitForExistence(timeout: 5))
        let center = diagram.buttons["buildhunter.diagram.chart.centerUp"]
        XCTAssertTrue(center.waitForExistence(timeout: 5))
        // Anchor to the actual center overlay: the chart's accessibility bounds can
        // include only its marks, rather than the complete square plot frame.
        // The fixture's Packages sector occupies the right side of the inner ring.
        center.coordinate(withNormalizedOffset: CGVector(dx: 1.25, dy: 0.5)).hover()
        expectValue("Packages", of: focus)
        XCTAssertTrue(diagram.buttons["buildhunter.diagram.folders.folder.Packages/Core"].exists)
        attachScreenshot(named: "diagram-hover-preview", from: app)
        diagram.staticTexts["buildhunter.diagram.target"].hover()
        expectValue("Demo Workspace", of: focus)
        let packages = diagram.buttons["buildhunter.diagram.folders.folder.Packages"]
        XCTAssertTrue(packages.waitForExistence(timeout: 5))
        packages.hover()
        attachScreenshot(named: "diagram-folder-hover", from: app)
        packages.click()
        expectValue("Packages", of: focus)
        XCTAssertTrue(center.isEnabled)
        center.click()
        expectValue("Demo Workspace", of: focus)
        XCTAssertFalse(center.isEnabled)
        // Synthetic reports deliberately have no filesystem URL to copy.
        XCTAssertFalse(diagram.buttons["buildhunter.diagram.copyPath"].isEnabled)
    }

    func testTableHeadersSortRowsInBothDirections() {
        let app = launchWindow()
        defer { app.terminate() }
        let window = app.windows.containing(.button, identifier: "buildhunter.toolbar.openDiagram").firstMatch
        selectMockState("results", in: window, app: app)
        // SwiftUI Table exposes an AXOutline on macOS, with each row combining
        // its cells into one accessibility label.
        let table = window.outlines.firstMatch
        XCTAssertTrue(table.waitForExistence(timeout: 5))
        let firstRow = table.tableRows.element(boundBy: 0)
        expectRowPath("Packages/Core/.build", of: firstRow)
        let pathHeader = table.buttons["Path"]
        XCTAssertTrue(pathHeader.waitForExistence(timeout: 5))
        pathHeader.click()
        expectRowPath("Tools/Indexer/target", of: firstRow)
        pathHeader.click()
        expectRowPath("Packages/Core/.build", of: firstRow)
        table.buttons["Size"].click()
        expectRowPath("Services/API/.pytest_cache", of: firstRow)
        table.buttons["Size"].click()
        expectRowPath("Packages/Core/.build", of: firstRow)
        table.buttons["Language"].click()
        expectRowPath("Services/API/.pytest_cache", of: firstRow)
        attachScreenshot(named: "report-sorted-columns-and-footer", from: app)
    }

    func testFullFolderPathCanBeCopiedAfterOpeningARealTarget() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("BuildHunter UI \(UUID().uuidString)", isDirectory: true)
            .resolvingSymlinksInPath()
        let cache = root.appendingPathComponent("Package/.build", isDirectory: true)
        try FileManager.default.createDirectory(at: cache, withIntermediateDirectories: true)
        try Data(repeating: 65, count: 4_096).write(to: cache.appendingPathComponent("fixture.o"))
        defer { try? FileManager.default.removeItem(at: root) }
        let app = launchWindow()
        defer { app.terminate() }
        app.buttons["buildhunter.empty.openFolder"].click()
        let open = app.descendants(matching: .any).matching(identifier: "OKButton").firstMatch
        XCTAssertTrue(open.waitForExistence(timeout: 5))
        app.typeKey("g", modifierFlags: [.command, .shift])
        app.typeText(root.path)
        app.typeKey(.return, modifierFlags: [])
        open.click()
        let scanWindow = app.windows.containing(.button, identifier: "buildhunter.toolbar.openDiagram").firstMatch
        expectValue("Scan complete", of: scanWindow.staticTexts["buildhunter.report.status"])
        scanWindow.buttons["buildhunter.toolbar.openDiagram"].click()
        let diagram = app.windows.containing(.staticText, identifier: "buildhunter.diagram.target").firstMatch
        XCTAssertTrue(diagram.waitForExistence(timeout: 10))
        let focus = diagram.staticTexts["buildhunter.diagram.focus"]
        expectValue(root.path, of: focus)
        diagram.buttons["buildhunter.diagram.folders.folder.Package"].click()
        let expected = root.appendingPathComponent("Package").path
        expectValue(expected, of: focus)
        diagram.buttons["buildhunter.diagram.copyPath"].click()
        let filter = diagram.textFields["buildhunter.diagram.filter"]
        filter.click()
        app.typeKey("v", modifierFlags: .command)
        XCTAssertEqual(filter.value as? String, expected)
        attachScreenshot(named: "diagram-full-path-copy", from: app)
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
        let windowMenu = app.menuBars.menuBarItems["Window"]
        windowMenu.click()
        let item = windowMenu.menuItems[title]
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

    private func expectRowPath(_ path: String, of row: XCUIElement,
                               file: StaticString = #filePath, line: UInt = #line) {
        let expectation = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label BEGINSWITH %@", path),
                                                    object: row)
        XCTAssertEqual(XCTWaiter.wait(for: [expectation], timeout: 5), .completed,
                       "Expected first row path: \(path); got \(row.debugDescription)", file: file, line: line)
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
