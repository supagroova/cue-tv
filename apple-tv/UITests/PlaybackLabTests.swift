import XCTest

nonisolated final class PlaybackLabTests: XCTestCase {
    @MainActor
    func testVLCNaturalEndReleasesPlayback() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_PLAYBACK_LAB"] = "1"
        app.launchEnvironment["CUE_VLC_LAB"] = "1"
        app.launch()
        let play = app.buttons["Play MKV with VLC"]
        XCTAssertTrue(play.waitForExistence(timeout: 10))
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.staticTexts["VLC playback advanced"].waitForExistence(timeout: 20))
        // The owned fixture is 12 seconds; completion must not wait for a stall timeout.
        XCTAssertTrue(play.waitForExistence(timeout: 15))
        XCTAssertFalse(app.staticTexts["Playback failed: read"].exists)
    }

    @MainActor
    func testVLCPauseSeekAndResume() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_PLAYBACK_LAB"] = "1"
        app.launchEnvironment["CUE_VLC_LAB"] = "1"
        app.launch()
        XCTAssertTrue(app.buttons["Play MKV with VLC"].waitForExistence(timeout: 10))
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.staticTexts["VLC playback advanced"].waitForExistence(timeout: 20))
        XCUIRemote.shared.press(.down)
        XCTAssertTrue(app.buttons["Pause"].hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.staticTexts["VLC paused"].waitForExistence(timeout: 5))
        let clock = app.staticTexts["vlc-time"]
        let pausedAt = try XCTUnwrap(Int(clock.label))
        let stable = NSPredicate { _, _ in abs((Int(clock.label) ?? -9999) - pausedAt) > 100 }
        let moved = XCTNSPredicateExpectation(predicate: stable, object: clock)
        moved.isInverted = true
        wait(for: [moved], timeout: 0.5)
        XCUIRemote.shared.press(.down)
        XCTAssertTrue(app.buttons["Seek to 8 seconds"].hasFocus)
        XCUIRemote.shared.press(.select)
        XCUIRemote.shared.press(.up)
        XCTAssertTrue(app.buttons["Resume"].hasFocus)
        XCUIRemote.shared.press(.select)
        let sought = NSPredicate { _, _ in (Int(clock.label) ?? 0) >= 8500 }
        expectation(for: sought, evaluatedWith: clock)
        waitForExpectations(timeout: 5)
    }

    @MainActor
    func testHEVCWithAlternateAudioAndSubtitles() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_PLAYBACK_LAB"] = "1"
        app.launchEnvironment["CUE_VLC_LAB"] = "1"
        app.launchEnvironment["CUE_VLC_MATRIX"] = "1"
        app.launch()
        let play = app.buttons["Play MKV with VLC"]
        XCTAssertTrue(play.waitForExistence(timeout: 10))
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.staticTexts["VLC playback advanced"].waitForExistence(timeout: 20))
        XCTAssertTrue(app.staticTexts["Audio tracks: 2; subtitle tracks: 2"].waitForExistence(timeout: 5))
        let audio = app.buttons["Next audio track"]
        XCUIRemote.shared.press(.down)
        XCTAssertTrue(audio.hasFocus)
        XCUIRemote.shared.press(.select)
        let subtitles = app.buttons["Next subtitle track"]
        XCUIRemote.shared.press(.down)
        XCTAssertTrue(subtitles.hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.staticTexts["Selected audio: 2; subtitle: 2"].waitForExistence(timeout: 5))
        let clock = app.staticTexts["vlc-time"]
        let selectedAt = Int(clock.label) ?? 0
        let nextCue = NSPredicate { _, _ in (Int(clock.label) ?? 0) >= selectedAt + 1200 }
        expectation(for: nextCue, evaluatedWith: clock)
        waitForExpectations(timeout: 4)
        let frame = XCTAttachment(screenshot: app.screenshot())
        frame.name = "HEVC with selected WebVTT subtitle"
        frame.lifetime = .keepAlways
        add(frame)
    }

    @MainActor
    func testTorrentPlaybackWithVLCCandidate() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_PLAYBACK_LAB"] = "1"
        app.launchEnvironment["CUE_VLC_LAB"] = "1"
        app.launch()
        let stream = app.buttons["Stream torrent with VLC"]
        XCTAssertTrue(stream.waitForExistence(timeout: 10))
        XCUIRemote.shared.press(.down)
        XCTAssertTrue(stream.hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.staticTexts["VLC playback advanced"].waitForExistence(timeout: 30))
        let stop = app.buttons["Stop VLC fixture"]
        XCTAssertTrue(stop.hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(stream.waitForExistence(timeout: 5))
    }

    @MainActor
    func testMKVPlaybackWithVLCCandidate() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_PLAYBACK_LAB"] = "1"
        app.launchEnvironment["CUE_VLC_LAB"] = "1"
        app.launch()
        let play = app.buttons["Play MKV with VLC"]
        XCTAssertTrue(play.waitForExistence(timeout: 10))
        XCTAssertTrue(play.hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.staticTexts["VLC playback advanced"].waitForExistence(timeout: 20))
        let frame = XCTAttachment(screenshot: app.screenshot())
        frame.name = "VLCKit MKV playback"
        frame.lifetime = .keepAlways
        add(frame)
        let stop = app.buttons["Stop VLC fixture"]
        XCTAssertTrue(stop.hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(play.waitForExistence(timeout: 5))
    }

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
