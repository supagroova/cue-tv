import XCTest
nonisolated final class ProbeTests: XCTestCase {
    @MainActor func testOwnedMediaMatrix() throws {
        for (fixture, alternate) in [("episode01.mp4", false), ("episode01.mkv", false), ("hevc-tracks.mkv", false), ("hevc-tracks.mkv", true)] {
            let app = XCUIApplication()
            app.launchEnvironment["CUE_MPV_FIXTURE"] = fixture
            app.launchEnvironment["CUE_MPV_ALTERNATE"] = alternate ? "1" : "0"
            app.launch()
            let clock = app.staticTexts["mpv-progress"]
            XCTAssertTrue(clock.waitForExistence(timeout: 10))
            let playing = NSPredicate { _, _ in (Double(clock.label) ?? 0) > 0.5 }
            expectation(for: playing, evaluatedWith: clock)
            waitForExpectations(timeout: 20)
            if alternate { XCTAssertTrue(app.staticTexts["Selected audio: 2; subtitle: 2"].exists) }
            let frame = XCTAttachment(screenshot: app.screenshot())
            frame.name = "MPV candidate: \(fixture), alternate=\(alternate)"
            frame.lifetime = .keepAlways
            add(frame)
            app.terminate()
        }
    }
}
