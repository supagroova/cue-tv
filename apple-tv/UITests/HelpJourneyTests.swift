import XCTest

/// Opt-in user-help navigation; no fixture account or media server required.
nonisolated final class HelpJourneyTests: XCTestCase {
    @MainActor func testHelpExplainsStorageAndSharingAndReturnsFocus() throws {
        let app = XCUIApplication()
        addTeardownBlock { @MainActor () async throws in app.terminate() }
        app.launch()
        let help = app.buttons["standalone-help"]
        XCTAssertTrue(help.waitForExistence(timeout: 10))
        for _ in 0..<16 where !help.hasFocus { XCUIRemote.shared.press(.down) }
        guard help.hasFocus else { XCTFail("Help did not receive remote focus"); return }
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.staticTexts["Using Cue TV"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["Storage"].exists)
        XCTAssertTrue(app.staticTexts["Torrent sharing"].exists)
        XCTAssertTrue(app.staticTexts["Account recovery"].exists)
        XCTAssertTrue(app.buttons["close-help"].exists)
        let capture = XCTAttachment(screenshot: app.screenshot())
        capture.name = "Standalone help"
        capture.lifetime = .keepAlways
        add(capture)
        XCUIRemote.shared.press(.menu)
        XCTAssertTrue(help.waitForExistence(timeout: 10))
        XCTAssertTrue(help.hasFocus)
        XCTAssertTrue(app.staticTexts["Stremio · Guest"].exists)
    }
}
