import XCTest

/// Opt-in: scripts/fixtures/serve-addon.py supplies real Stremio protocol resources.
nonisolated final class StandaloneJourneyTests: XCTestCase {
    @MainActor func testBackRestoresSelectedEpisodeAndTitle() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_JOURNEY_FIXTURE"] = "1"
        app.launch()
        let catalog = app.buttons["Owned series · series"]
        XCTAssertTrue(catalog.waitForExistence(timeout: 20))
        for _ in 0..<20 where !catalog.hasFocus { XCUIRemote.shared.press(.down) }
        XCTAssertTrue(catalog.hasFocus)
        XCUIRemote.shared.press(.select)
        let title = app.buttons["Owned series 2"]
        XCTAssertTrue(title.waitForExistence(timeout: 5))
        for _ in 0..<8 where !title.hasFocus { XCUIRemote.shared.press(.down) }
        XCTAssertTrue(title.hasFocus)
        XCUIRemote.shared.press(.select)
        let episode = app.buttons["S2 · E2 · Episode 2"]
        XCTAssertTrue(episode.waitForExistence(timeout: 5))
        for _ in 0..<12 where !episode.hasFocus { XCUIRemote.shared.press(.down) }
        XCTAssertTrue(episode.hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.buttons["Owned peer fixture"].waitForExistence(timeout: 5))
        XCUIRemote.shared.press(.menu)
        XCTAssertTrue(episode.waitForExistence(timeout: 5))
        XCTAssertTrue(episode.hasFocus, "Back must restore the selected stable episode identity")
        XCUIRemote.shared.press(.menu)
        XCTAssertTrue(title.waitForExistence(timeout: 5))
        XCTAssertTrue(title.hasFocus, "Back must restore the selected title")
        XCUIRemote.shared.press(.menu)
        XCTAssertTrue(catalog.waitForExistence(timeout: 5))
        XCTAssertTrue(catalog.hasFocus, "Back must restore the selected catalog")
        let frame = XCTAttachment(screenshot: app.screenshot())
        frame.name = "Restored catalog focus"
        frame.lifetime = .keepAlways
        add(frame)
    }
}
