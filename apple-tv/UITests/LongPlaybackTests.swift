import XCTest

/// Explicit long-running qualification; generate the optional fixture and start its seeder first.
nonisolated final class LongPlaybackTests: XCTestCase {
    @MainActor
    func testOneMinuteVLCAndDistantSeek() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_PLAYBACK_LAB"] = "1"
        app.launchEnvironment["CUE_VLC_LAB"] = "1"
        app.launchEnvironment["CUE_LONG_TORRENT"] = "1"
        app.launchEnvironment["CUE_VLC_LONG_SEEK"] = "1"
        app.launch()
        let stream = app.buttons["Stream torrent with VLC"]
        XCTAssertTrue(stream.waitForExistence(timeout: 10))
        XCUIRemote.shared.press(.down)
        XCTAssertTrue(stream.hasFocus)
        XCUIRemote.shared.press(.select)
        let evidence = app.staticTexts["vlc-first-progress"]
        XCTAssertTrue(evidence.waitForExistence(timeout: 40))
        let values = evidence.label.components(separatedBy: CharacterSet.decimalDigits.inverted).compactMap(UInt64.init)
        XCTAssertEqual(values.count, 2)
        XCTAssertGreaterThan(values[0], 0)
        XCTAssertLessThan(values[0], values[1])
        let clock = app.staticTexts["vlc-time"]
        expectation(for: NSPredicate { _, _ in (Int(clock.label) ?? 0) >= 60000 }, evaluatedWith: clock)
        waitForExpectations(timeout: 90)
        XCUIRemote.shared.press(.down)
        XCUIRemote.shared.press(.down)
        XCTAssertTrue(app.buttons["Seek to 600 seconds"].hasFocus)
        XCUIRemote.shared.press(.select)
        expectation(for: NSPredicate { _, _ in (Int(clock.label) ?? 0) >= 600500 }, evaluatedWith: clock)
        waitForExpectations(timeout: 15)
        let frame = XCTAttachment(screenshot: app.screenshot())
        frame.name = evidence.label + " after distant seek"
        frame.lifetime = .keepAlways
        add(frame)
        XCUIRemote.shared.press(.up)
        XCUIRemote.shared.press(.up)
        XCTAssertTrue(app.buttons["Stop VLC fixture"].hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(stream.waitForExistence(timeout: 5))
    }

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
