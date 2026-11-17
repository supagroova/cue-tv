import XCTest
import CoreGraphics

/// Opt-in: scripts/fixtures/serve-addon.py supplies real Stremio protocol resources.
nonisolated final class StandaloneJourneyTests: XCTestCase {
    @MainActor func testBackRestoresSelectedEpisodeAndTitle() throws { try journey(play: false) }
    @MainActor func testSelectedEpisodePlaysFromPeerAndCloses() throws { try journey(play: true) }
    @MainActor private func journey(play: Bool) throws {
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
        XCTAssertTrue(app.buttons["Owned peer fixture cue-owned-2:2:2"].waitForExistence(timeout: 5))
        if play {
            let source = app.buttons["Owned peer fixture cue-owned-2:2:2"]
            for _ in 0..<8 where !source.hasFocus { XCUIRemote.shared.press(.down) }
            XCTAssertTrue(source.hasFocus)
            XCUIRemote.shared.press(.select)
            let clock = app.staticTexts["source-playback-time"]
            XCTAssertTrue(clock.waitForExistence(timeout: 25))
            expectation(for: NSPredicate { _, _ in clock.exists && (Int(clock.label.dropLast()) ?? 0) >= 3 }, evaluatedWith: nil)
            waitForExpectations(timeout: 20)
            XCTAssertFalse(app.staticTexts["source-playback-error"].exists)
            let screenshot = app.screenshot()
            // Owned media encode episode one blue, episode two green, and the tail-index file red.
            // Inspect decoded pixels, independently of source labels and the requested file index.
            let image = try XCTUnwrap(screenshot.image.cgImage)
            let pixel = try XCTUnwrap(image.cropping(to: CGRect(x: image.width / 2, y: image.height / 3, width: 1, height: 1)))
            let context = try XCTUnwrap(CGContext(data: nil, width: 1, height: 1, bitsPerComponent: 8, bytesPerRow: 4,
                space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue))
            context.draw(pixel, in: CGRect(x: 0, y: 0, width: 1, height: 1))
            let rgba = try XCTUnwrap(context.data).assumingMemoryBound(to: UInt8.self)
            XCTAssertLessThan(rgba[0], 40, "Episode two must not show the red tail-index file")
            XCTAssertGreaterThan(rgba[1], 70, "Episode two must show its green decoded frame")
            XCTAssertLessThan(rgba[2], 40, "Episode two must not show the blue first episode")
            let frame = XCTAttachment(screenshot: screenshot)
            frame.name = "Selected episode two peer playback"
            frame.lifetime = .keepAlways
            add(frame)
            XCTAssertTrue(app.buttons["Close"].hasFocus)
            XCUIRemote.shared.press(.select)
            XCTAssertTrue(source.waitForExistence(timeout: 5))
            expectation(for: NSPredicate { _, _ in source.hasFocus }, evaluatedWith: nil)
            waitForExpectations(timeout: 5)
            XCTAssertTrue(source.hasFocus, "Closing playback restores the chosen source")
        }
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
