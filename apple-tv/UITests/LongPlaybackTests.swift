import XCTest

/// Explicit long-running qualification; generate the optional fixture and start its seeder first.
nonisolated final class LongPlaybackTests: XCTestCase {
    @MainActor
    func testTenMinutesOfDirectPlayback() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_PLAYBACK_LAB"] = "1"
        app.launchEnvironment["CUE_LONG_TORRENT"] = "1"
        app.launch()
        let stream = app.buttons["Stream torrent fixture"]
        XCTAssertTrue(stream.waitForExistence(timeout: 10))
        for _ in 0..<8 {
            if stream.hasFocus { break }
            XCUIRemote.shared.press(.down)
        }
        XCTAssertTrue(stream.hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.staticTexts["Playback advanced"].waitForExistence(timeout: 40))
        let evidence = app.staticTexts["first-frame-evidence"]
        XCTAssertTrue(evidence.waitForExistence(timeout: 5))
        let values = evidence.label.components(separatedBy: CharacterSet.decimalDigits.inverted).compactMap(UInt64.init)
        XCTAssertEqual(values.count, 2)
        XCTAssertGreaterThan(values[0], 0)
        XCTAssertLessThan(values[0], values[1], "playback must start before download completion")
        let start = XCTAttachment(screenshot: app.screenshot())
        start.name = evidence.label
        start.lifetime = .keepAlways
        add(start)
        let elapsed = app.staticTexts["playback-seconds"]
        let advanced = NSPredicate { _, _ in Double(elapsed.label) ?? 0 >= 600 }
        expectation(for: advanced, evaluatedWith: elapsed)
        waitForExpectations(timeout: 660)
        let end = XCTAttachment(screenshot: app.screenshot())
        end.name = "Ten minutes of direct playback"
        end.lifetime = .keepAlways
        add(end)
    }
}
