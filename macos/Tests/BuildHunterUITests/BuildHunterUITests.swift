import XCTest

@MainActor
final class BuildHunterUITests: XCTestCase {
    func testSettingsWindowPersistsSearchSelection() {
        let suite = "BuildHunter.SettingsUITests.\(UUID().uuidString)"
        var app = launchWindow(settingsSuite: suite)
        openSettings(app)
        let filterID = "buildhunter.settings.search.filter.swift.build"
        let swiftFilter = app.switches[filterID]
        XCTAssertTrue(swiftFilter.waitForExistence(timeout: 5), "The application menu must open the native Settings window")
        XCTAssertEqual((swiftFilter.value as? NSNumber)?.intValue, 1)
        swiftFilter.click()
        XCTAssertEqual((swiftFilter.value as? NSNumber)?.intValue, 0)
        attachScreenshot(named: "settings-search-exclusions", from: app)
        app.terminate()

        app = launchWindow(settingsSuite: suite)
        defer { app.terminate() }
        openSettings(app)
        let persisted = app.switches[filterID]
        XCTAssertTrue(persisted.waitForExistence(timeout: 5))
        XCTAssertEqual((persisted.value as? NSNumber)?.intValue, 0, "Search exclusions must persist across application launches")
        let enableAll = app.buttons["buildhunter.settings.search.enableAll"]
        XCTAssertTrue(enableAll.waitForExistence(timeout: 5))
        enableAll.click()
        XCTAssertEqual((persisted.value as? NSNumber)?.intValue, 1)
    }

    func testScanProfilePanelExpandsAndCollapses() {
        let app = launchWindow()
        defer { app.terminate() }
        let window = app.windows.containing(.button, identifier: "buildhunter.toolbar.openDiagram").firstMatch
        XCTAssertTrue(window.waitForExistence(timeout: 5))
        selectMockState("results", in: window, app: app)
        let chart = app.descendants(matching: .any)
            .matching(identifier: "buildhunter.report.profile.chart").firstMatch
        XCTAssertFalse(chart.exists, "The profile panel starts collapsed")
        let disclosure = app.descendants(matching: .any)
            .matching(identifier: "buildhunter.report.profile").firstMatch
        XCTAssertTrue(disclosure.waitForExistence(timeout: 5))
        clickDisclosureTriangle(disclosure)
        XCTAssertTrue(chart.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["buildhunter.report.profile.average"].exists)
        XCTAssertTrue(app.staticTexts["buildhunter.report.profile.peak"].exists)
        attachScreenshot(named: "scan-profile-completed", from: app)
        clickDisclosureTriangle(disclosure)
        let collapsed = XCTNSPredicateExpectation(
            predicate: NSPredicate(format: "exists == false"), object: chart)
        wait(for: [collapsed], timeout: 5)
    }

    private func openSettings(_ app: XCUIApplication) {
        app.activate()
        app.menuBars.menuBarItems["BuildHunter"].click()
        let menuItem = app.menuItems["Settings…"]
        XCTAssertTrue(menuItem.waitForExistence(timeout: 5))
        menuItem.click()
    }

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

    func testDiagramTopologyChangesDoNotCrash() {
        let app = launchWindow()
        defer { app.terminate() }
        let scanWindow = app.windows.containing(.button, identifier: "buildhunter.toolbar.openDiagram").firstMatch
        selectMockState("results", in: scanWindow, app: app)
        scanWindow.buttons["buildhunter.toolbar.openDiagram"].click()
        let diagram = app.windows.containing(.staticText, identifier: "buildhunter.diagram.target").firstMatch
        XCTAssertTrue(diagram.waitForExistence(timeout: 10))
        for _ in 0..<3 {
            for scenario in ["scanning", "results", "stopped"] {
                selectMockState(scenario, in: scanWindow, app: app)
                activateWindow(titled: "Demo Workspace — Artifact Diagram", in: app)
                let contentID = scenario == "scanning" ? "buildhunter.diagram.emptyStatus" : "buildhunter.diagram.chart"
                XCTAssertTrue(diagram.descendants(matching: .any).matching(identifier: contentID)
                    .firstMatch.waitForExistence(timeout: 5))
                // Capturing forces the updated chart through its Canvas render pass.
                _ = diagram.screenshot()
                XCTAssertEqual(app.state, .runningForeground, "Streaming topology changes must not crash Charts")
            }
        }
        attachScreenshot(named: "diagram-topology-regression", from: app)
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
        // SwiftUI Table exposes an AXOutline on macOS. Read the Path cell
        // within its first row rather than relying on a combined row label.
        let table = window.outlines.firstMatch
        XCTAssertTrue(table.waitForExistence(timeout: 5))
        let firstRow = table.outlineRows.element(boundBy: 0)
        expectRowPath("Packages/Core/.build", of: firstRow)
        let pathHeader = table.buttons["Path"]
        XCTAssertTrue(pathHeader.waitForExistence(timeout: 5))
        clickTableHeader(pathHeader)
        expectRowPath("Tools/Indexer/target", of: firstRow)
        clickTableHeader(pathHeader)
        expectRowPath("Packages/Core/.build", of: firstRow)
        clickTableHeader(table.buttons["Size"])
        expectRowPath("Services/API/.pytest_cache", of: firstRow)
        clickTableHeader(table.buttons["Size"])
        expectRowPath("Packages/Core/.build", of: firstRow)
        clickTableHeader(table.buttons["Language"])
        expectRowPath("Services/API/.pytest_cache", of: firstRow)
        firstRow.hover()
        attachScreenshot(named: "report-sorted-columns-and-footer", from: app)
    }

    func testFullFolderPathCanBeCopiedAfterOpeningARealTarget() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("BuildHunter UI \(UUID().uuidString)", isDirectory: true)
        let cache = root.appendingPathComponent("Package/.build", isDirectory: true)
        try FileManager.default.createDirectory(at: cache, withIntermediateDirectories: true)
        try Data(repeating: 65, count: 4_096).write(to: cache.appendingPathComponent("fixture.o"))
        defer { try? FileManager.default.removeItem(at: root) }
        // realpath can resolve /var -> /private/var only after the fixture exists.
        let selectedRoot = root.resolvingSymlinksInPath()
        let app = launchWindow()
        defer { app.terminate() }
        app.buttons["buildhunter.empty.openFolder"].click()
        let open = app.descendants(matching: .any).matching(identifier: "OKButton").firstMatch
        XCTAssertTrue(open.waitForExistence(timeout: 5))
        app.typeKey("g", modifierFlags: [.command, .shift])
        app.typeText(selectedRoot.path)
        app.typeKey(.return, modifierFlags: [])
        open.click()
        let scanWindow = app.windows.containing(.button, identifier: "buildhunter.toolbar.openDiagram").firstMatch
        expectValue("Scan complete", of: scanWindow.staticTexts["buildhunter.report.status"])
        scanWindow.buttons["buildhunter.toolbar.openDiagram"].click()
        let diagram = app.windows.containing(.staticText, identifier: "buildhunter.diagram.target").firstMatch
        XCTAssertTrue(diagram.waitForExistence(timeout: 10))
        let focus = diagram.staticTexts["buildhunter.diagram.focus"]
        expectFilesystemPath(selectedRoot.path, of: focus)
        diagram.buttons["buildhunter.diagram.folders.folder.Package"].click()
        expectFilesystemPath(selectedRoot.appendingPathComponent("Package").path, of: focus)
        // Copy must preserve exactly the absolute path shown by the app.
        let expected = try XCTUnwrap(focus.value as? String)
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

    private func clickTableHeader(_ header: XCUIElement) {
        // SwiftUI Table's AX button can be marked not hittable on macOS 26
        // even while visibly laid out. Clicking its center avoids XCTest's
        // incorrect automatic ScrollView repositioning of the header.
        XCTAssertTrue(header.waitForExistence(timeout: 5))
        header.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
    }

    private func clickDisclosureTriangle(_ disclosure: XCUIElement) {
        // On macOS the DisclosureGroup AX element spans the triangle and its label,
        // but only the leading triangle toggles. A center click lands on the label.
        disclosure.coordinate(withNormalizedOffset: CGVector(dx: 0, dy: 0.5))
            .withOffset(CGVector(dx: 10, dy: 0)).click()
    }

    private func expectRowPath(_ path: String, of row: XCUIElement,
                               file: StaticString = #filePath, line: UInt = #line) {
        let cell = row.descendants(matching: .any).matching(
            NSPredicate(format: "value == %@ OR label BEGINSWITH %@", path, path)
        ).firstMatch
        XCTAssertTrue(cell.waitForExistence(timeout: 5),
                      "Expected first row Path cell: \(path); got \(row.debugDescription)", file: file, line: line)
    }

    private func expectFilesystemPath(_ path: String, of element: XCUIElement,
                                      file: StaticString = #filePath, line: UInt = #line) {
        // NSOpenPanel may return either spelling of macOS's /var symlink.
        // Both are absolute paths to the same fixture; compare the text shown
        // exactly when checking the Copy action below.
        let alternate = path.hasPrefix("/private/var/")
            ? String(path.dropFirst("/private".count))
            : (path.hasPrefix("/var/") ? "/private" + path : path)
        let expectation = XCTNSPredicateExpectation(
            predicate: NSPredicate(format: "value == %@ OR value == %@", path, alternate), object: element)
        XCTAssertEqual(XCTWaiter.wait(for: [expectation], timeout: 5), .completed,
                       "Expected absolute fixture path: \(path); got \(element.value ?? "nil")", file: file, line: line)
    }

    private func launchWindow(settingsSuite: String = "BuildHunter.UITests.\(UUID().uuidString)") -> XCUIApplication {
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launchEnvironment["BUILDHUNTER_SETTINGS_SUITE"] = settingsSuite
        // Register cleanup before asserting, so a failed launch cannot leave
        // a windowless process behind for the next test.
        addTeardownBlock { app.terminate() }
        app.launch()
        app.activate()

        // Relaunch can leave a macOS multiwindow app running without a window.
        // Invoke the actual menu command: a keyboard shortcut can be consumed
        // by another app or a global shortcut on the developer's desktop.
        if !app.windows.firstMatch.waitForExistence(timeout: 5) {
            app.menuBars.menuBarItems["File"].click()
            let newWindow = app.menuItems["New BuildHunter Window"]
            XCTAssertTrue(newWindow.waitForExistence(timeout: 5))
            newWindow.click()
        }
        let hasWindow = app.windows.firstMatch.waitForExistence(timeout: 5)
        if !hasWindow {
            attachScreenshot(named: "launch-missing-window", from: app)
            let hierarchy = XCTAttachment(string: app.debugDescription)
            hierarchy.name = "launch-accessibility-hierarchy"
            hierarchy.lifetime = .keepAlways
            add(hierarchy)
        }
        XCTAssertTrue(hasWindow,
                      "A BuildHunter window must exist before testing its controls; app state: \(app.state)")
        return app
    }

    private func attachScreenshot(named name: String, from app: XCUIApplication) {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
