import XCTest

/// Opt-in transport proof; requires the isolated TLS fixture server and simulator trust setup.
nonisolated final class HTTPSPlaybackTests: XCTestCase {
    @MainActor func testTrustedHTTPSAdvancesPausesSeeksAndCloses() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_HTTPS_TEST_FIXTURE"] = "1"
        app.launch()
        let clock = app.staticTexts["source-playback-time"]
        XCTAssertTrue(clock.waitForExistence(timeout: 15))
        let advanced = NSPredicate { _, _ in (Int(clock.label.dropLast()) ?? 0) >= 5 }
        expectation(for: advanced, evaluatedWith: nil)
        waitForExpectations(timeout: 20)
        XCTAssertFalse(app.staticTexts["source-playback-error"].exists)
        let frame = XCTAttachment(screenshot: app.screenshot())
        frame.name = "Trusted HTTPS playback"
        frame.lifetime = .keepAlways
        add(frame)
        let close = app.buttons["Close"]
        XCTAssertTrue(close.hasFocus)
        XCUIRemote.shared.press(.up)
        let pause = app.buttons["Pause"]
        for _ in 0..<4 where !pause.hasFocus { XCUIRemote.shared.press(.left) }
        XCTAssertTrue(pause.hasFocus)
        XCUIRemote.shared.press(.select)
        let resume = app.buttons["Resume"]
        XCTAssertTrue(resume.waitForExistence(timeout: 3))
        let pausedAt = clock.label
        let moved = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in clock.label != pausedAt }, object: nil)
        moved.isInverted = true
        wait(for: [moved], timeout: 0.5)
        XCUIRemote.shared.press(.right)
        XCTAssertTrue(app.buttons["Back 10 seconds"].hasFocus)
        XCUIRemote.shared.press(.select)
        expectation(for: NSPredicate { _, _ in clock.label == "0s" }, evaluatedWith: nil)
        waitForExpectations(timeout: 5)
        XCUIRemote.shared.press(.left)
        XCTAssertTrue(resume.hasFocus)
        XCUIRemote.shared.press(.select)
        expectation(for: NSPredicate { _, _ in (Int(clock.label.dropLast()) ?? 0) >= 1 }, evaluatedWith: nil)
        waitForExpectations(timeout: 5)
        XCUIRemote.shared.press(.down)
        XCTAssertTrue(close.hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.buttons["Sign in to Stremio"].waitForExistence(timeout: 5))
        XCTAssertFalse(clock.exists)
    }
    @MainActor func testUntrustedHTTPSCannotStartPlayback() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_HTTPS_TEST_FIXTURE"] = "1"
        app.launchEnvironment["CUE_HTTPS_TEST_UNTRUSTED"] = "1"
        app.launch()
        let failure = app.staticTexts["source-playback-error"]
        let clock = app.staticTexts["source-playback-time"]
        let outcome = NSPredicate { _, _ in
            failure.exists || (clock.exists && (Int(clock.label.dropLast()) ?? 0) > 0)
        }
        expectation(for: outcome, evaluatedWith: nil)
        waitForExpectations(timeout: 35)
        XCTAssertTrue(failure.exists, "An untrusted certificate must never produce advancing video")
        XCTAssertFalse(clock.exists)
    }
}
