import XCTest

nonisolated final class PlaybackLabTests: XCTestCase {
    @MainActor
    func testTorrentFixtureAdvancesPlayback() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_PLAYBACK_LAB"] = "1"
        app.launch()
        let stream = app.buttons["Stream torrent fixture"]
        XCTAssertTrue(stream.waitForExistence(timeout: 10))
        for _ in 0..<8 {
            if stream.hasFocus { break }
            XCUIRemote.shared.press(.down)
        }
        XCTAssertTrue(stream.hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.staticTexts["Playback advanced"].waitForExistence(timeout: 30))
        let screenshot = XCTAttachment(screenshot: app.screenshot())
        screenshot.name = "Direct torrent playback in simulator"
        screenshot.lifetime = .keepAlways
        add(screenshot)
    }

    @MainActor func testLocalFixtureStartsAndStopsWithoutPairing() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_PLAYBACK_LAB"] = "1"
        app.launch()
        let first = app.buttons["Play Episode01 — 12s"]
        XCTAssertTrue(first.waitForExistence(timeout: 10))
        XCTAssertTrue(first.hasFocus)
        XCUIRemote.shared.press(.select)
        let stop = app.buttons["Stop playback"]
        XCTAssertTrue(stop.waitForExistence(timeout: 10))
        for _ in 0..<8 {
            if stop.hasFocus { break }
            XCUIRemote.shared.press(.down)
        }
        XCTAssertTrue(stop.hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(first.waitForExistence(timeout: 5))
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = "Playback Lab after stop"
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
