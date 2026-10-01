import XCTest

@MainActor
final class BuildHunterUITests: XCTestCase {
    func testMockStatesAndCaptureScreenshots() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.buttons["buildhunter.empty.openFolder"].waitForExistence(timeout: 10))
        attachScreenshot(named: "01-empty", from: app)

        let scenarios: [(id: String, status: String)] = [
            ("scanning", "Showing simulated streaming results"),
            ("results", "Demo complete"),
            ("stopped", "Demo stopped · partial results"),
            ("incomplete", "Demo incomplete · review warnings")
        ]

        for (index, scenario) in scenarios.enumerated() {
            let mockStateMenu = app.descendants(matching: .any)
                .matching(identifier: "buildhunter.toolbar.mockState").firstMatch
            XCTAssertTrue(mockStateMenu.waitForExistence(timeout: 5))
            mockStateMenu.click()

            let optionID = "buildhunter.toolbar.mockState.scenario.\(scenario.id)"
            let option = app.descendants(matching: .any).matching(identifier: optionID).firstMatch
            XCTAssertTrue(option.waitForExistence(timeout: 5), "Missing mock-state option: \(optionID)")
            option.click()

            let status = app.staticTexts["buildhunter.report.status"]
            XCTAssertTrue(status.waitForExistence(timeout: 5))
            XCTAssertEqual(status.label, scenario.status)
            attachScreenshot(named: String(format: "%02d-%@", index + 2, scenario.id), from: app)
        }

        app.terminate()
    }

    func testOpenFolderPresentsAndDismissesPicker() {
        let app = XCUIApplication()
        app.launch()

        let openFolder = app.buttons["buildhunter.empty.openFolder"]
        XCTAssertTrue(openFolder.waitForExistence(timeout: 10))
        openFolder.click()

        let cancel = app.descendants(matching: .any)
            .matching(identifier: "CancelButton").firstMatch
        XCTAssertTrue(cancel.waitForExistence(timeout: 10), "Open Folder should present a folder picker")
        attachScreenshot(named: "folder-picker", from: app)
        cancel.click()

        XCTAssertTrue(openFolder.waitForExistence(timeout: 5), "Cancel should return to the empty window")
        app.terminate()
    }

    private func attachScreenshot(named name: String, from app: XCUIApplication) {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
