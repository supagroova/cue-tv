import XCTest

/// Opt-in transport proof; requires the isolated TLS fixture server and simulator trust setup.
nonisolated final class HTTPSPlaybackTests: XCTestCase {
    @MainActor func testTrustedHTTPSAdvancesAndCloses() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_HTTPS_TEST_FIXTURE"] = "1"
        app.launch()
        let clock = app.staticTexts["source-playback-time"]
        XCTAssertTrue(clock.waitForExistence(timeout: 15))
        let advanced = NSPredicate { _, _ in (Int(clock.label.dropLast()) ?? 0) >= 2 }
        expectation(for: advanced, evaluatedWith: nil)
        waitForExpectations(timeout: 20)
        XCTAssertFalse(app.staticTexts["source-playback-error"].exists)
        let close = app.buttons["Close"]
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
