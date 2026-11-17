import XCTest
import CoreGraphics

/// Opt-in: scripts/fixtures/serve-addon.py supplies real Stremio protocol resources.
nonisolated final class StandaloneJourneyTests: XCTestCase {
    @MainActor func testBackRestoresSelectedEpisodeAndTitle() throws { try journey(play: false) }
    @MainActor func testSelectedEpisodePlaysFromPeerAndCloses() throws { try journey(play: true) }
    @MainActor func testStreamSubtitleAppearsAndCanBeTurnedOff() throws { try journey(play: true, captions: true) }
    @MainActor private func journey(play: Bool, captions: Bool = false) throws {
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
        XCTAssertTrue(app.staticTexts["Cue journey fixture"].exists, "Sources identify their supplying add-on")
        XCTAssertTrue(app.staticTexts["Torrent · Size unknown"].exists)
        XCTAssertTrue(app.staticTexts["Owned synthetic H.264/AAC"].exists)
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
            if captions {
                XCUIRemote.shared.press(.up)
                let english = app.buttons["Owned English"]
                for _ in 0..<4 where !english.hasFocus { XCUIRemote.shared.press(.right) }
                XCTAssertTrue(english.hasFocus)
                XCUIRemote.shared.press(.select)
                let hasCaption = NSPredicate { _, _ in ((try? self.brightVideoPixels(app)) ?? 0) > 20 }
                expectation(for: hasCaption, evaluatedWith: nil)
                waitForExpectations(timeout: 5)
                let subtitleFrame = XCTAttachment(screenshot: app.screenshot())
                subtitleFrame.name = "Rendered owned subtitle"
                subtitleFrame.lifetime = .keepAlways
                add(subtitleFrame)
                XCUIRemote.shared.press(.left)
                XCTAssertTrue(app.buttons["Subtitles off"].hasFocus)
                XCUIRemote.shared.press(.select)
                expectation(for: NSPredicate { _, _ in (try? self.brightVideoPixels(app)) == 0 }, evaluatedWith: nil)
                waitForExpectations(timeout: 5)
                let positionBeforeReselect = Int(clock.label.dropLast()) ?? 0
                XCUIRemote.shared.press(.right)
                XCTAssertTrue(english.hasFocus)
                XCUIRemote.shared.press(.select)
                expectation(for: hasCaption, evaluatedWith: nil)
                waitForExpectations(timeout: 5)
                XCTAssertGreaterThanOrEqual(Int(clock.label.dropLast()) ?? 0, positionBeforeReselect - 1)
                XCUIRemote.shared.press(.down)
            }
            XCTAssertTrue(app.buttons["Close"].hasFocus)
            XCUIRemote.shared.press(captions ? .menu : .select)
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
    @MainActor private func brightVideoPixels(_ app: XCUIApplication) throws -> Int {
        let surface = app.descendants(matching: .any)["vlc-video-surface"].firstMatch
        guard surface.exists else { return -1 }
        let frame = surface.frame
        let screenshot = XCUIScreen.main.screenshot().image
        let image = try XCTUnwrap(screenshot.cgImage)
        let scale = screenshot.scale
        let rect = CGRect(x: frame.minX * scale, y: frame.minY * scale, width: frame.width * scale, height: frame.height * scale)
        let video = try XCTUnwrap(image.cropping(to: rect))
        let context = try XCTUnwrap(CGContext(data: nil, width: 480, height: 270, bitsPerComponent: 8, bytesPerRow: 1920,
            space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue))
        context.draw(video, in: CGRect(x: 0, y: 0, width: 480, height: 270))
        let rgba = try XCTUnwrap(context.data).assumingMemoryBound(to: UInt8.self)
        return (0..<(480 * 270)).filter { pixel in
            let base = pixel * 4
            return rgba[base] > 180 && rgba[base + 1] > 180 && rgba[base + 2] > 180
        }.count
    }

}
