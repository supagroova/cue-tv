import XCTest
import CoreGraphics
import CoreFoundation

/// Opt-in: scripts/fixtures/serve-addon.py supplies real Stremio protocol resources.
nonisolated final class StandaloneJourneyTests: XCTestCase {
    @MainActor func testBackRestoresSelectedEpisodeAndTitle() throws { try journey(play: false) }
    @MainActor func testSelectedEpisodePlaysFromPeerAndCloses() throws { try journey(play: true, resume: true) }
    @MainActor func testHomeInterruptionReturnsToSourceAndPreservesResume() throws { try journey(play: true, resume: true, background: true) }
    @MainActor func testPeriodicProgressSurvivesAbruptTermination() throws { try journey(play: true, resume: true, relaunch: true, abrupt: true) }
    @MainActor func testProgressSurvivesProcessRelaunch() throws { try journey(play: true, resume: true, relaunch: true) }
    @MainActor func testStreamSubtitleAppearsAndCanBeTurnedOff() throws { try journey(play: true, captions: true) }
    @MainActor func testSeasonFilterPreservesEpisodeSelectionAndBackFocus() throws { try journey(play: false, filterSeason: true) }
    @MainActor func testMovieWithoutEpisodesChoosesSourceAndRestoresFocus() throws { try movieJourney(library: false) }
    @MainActor func testMovieLibraryMembershipCanBeAddedAndRemoved() throws { try movieJourney(library: true) }
    @MainActor func testSavedFilmReopensFromLibraryAndRemovalUpdatesList() throws { try movieJourney(library: true, browseLibrary: true) }
    @MainActor func testFilmWatchedStateCanBeChangedAndSurvivesReturningToDetails() throws { try movieJourney(library: false, watched: true) }
    @MainActor private func movieJourney(library: Bool, browseLibrary: Bool = false, watched: Bool = false) throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_JOURNEY_FIXTURE"] = "1"
        app.launch()
        let catalog = app.buttons["Owned films · movie"]
        XCTAssertTrue(catalog.waitForExistence(timeout: 20))
        for _ in 0..<20 where !catalog.hasFocus { XCUIRemote.shared.press(.down) }
        XCTAssertTrue(catalog.hasFocus)
        XCUIRemote.shared.press(.select)
        let title = app.buttons["Owned film"]
        XCTAssertTrue(title.waitForExistence(timeout: 5))
        for _ in 0..<8 where !title.hasFocus { XCUIRemote.shared.press(.down) }
        XCTAssertTrue(title.hasFocus)
        XCUIRemote.shared.press(.select)
        if library {
            let add = app.buttons["Add to library"]
            XCTAssertTrue(add.waitForExistence(timeout: 5))
            for _ in 0..<8 where !add.hasFocus { XCUIRemote.shared.press(.down) }
            XCTAssertTrue(add.hasFocus)
            XCUIRemote.shared.press(.select)
            if browseLibrary {
                XCTAssertTrue(app.buttons["Remove from library"].waitForExistence(timeout: 5))
                XCUIRemote.shared.press(.menu)
                XCTAssertTrue(title.waitForExistence(timeout: 5))
                XCUIRemote.shared.press(.menu)
                XCTAssertTrue(catalog.waitForExistence(timeout: 5))
                XCTAssertTrue(catalog.hasFocus)
                let myLibrary = app.buttons["My library"]
                XCTAssertTrue(myLibrary.waitForExistence(timeout: 5))
                for _ in 0..<20 where !myLibrary.hasFocus { XCUIRemote.shared.press(.up) }
                XCTAssertTrue(myLibrary.hasFocus)
                XCUIRemote.shared.press(.select)
                XCTAssertTrue(title.waitForExistence(timeout: 5))
                for _ in 0..<8 where !title.hasFocus { XCUIRemote.shared.press(.down) }
                XCTAssertTrue(title.hasFocus)
                XCUIRemote.shared.press(.select)
                XCTAssertTrue(app.staticTexts["Owned film without episode metadata"].waitForExistence(timeout: 5))
            }
            let remove = app.buttons["Remove from library"]
            XCTAssertTrue(remove.waitForExistence(timeout: 5))
            for _ in 0..<8 where !remove.hasFocus { XCUIRemote.shared.press(.down) }
            XCTAssertTrue(remove.hasFocus)
            XCUIRemote.shared.press(.select)
            XCTAssertTrue(add.waitForExistence(timeout: 5))
            if browseLibrary {
                XCUIRemote.shared.press(.menu)
                XCTAssertTrue(app.staticTexts["Your library is empty. Add a title from its details."].waitForExistence(timeout: 5))
                XCTAssertFalse(title.exists)
                XCUIRemote.shared.press(.menu)
                let myLibrary = app.buttons["My library"]
                XCTAssertTrue(myLibrary.waitForExistence(timeout: 5))
                XCTAssertTrue(myLibrary.hasFocus)
                return
            }
        }
        if watched {
            let mark = app.buttons["Mark as watched"]
            XCTAssertTrue(mark.waitForExistence(timeout: 5))
            for _ in 0..<8 where !mark.hasFocus { XCUIRemote.shared.press(.down) }
            XCTAssertTrue(mark.hasFocus)
            XCUIRemote.shared.press(.select)
            let unmark = app.buttons["Mark as unwatched"]
            XCTAssertTrue(unmark.waitForExistence(timeout: 5))
            XCTAssertTrue(app.buttons["Add to library"].exists)
            XCUIRemote.shared.press(.menu)
            XCTAssertTrue(title.waitForExistence(timeout: 5))
            XCTAssertTrue(title.hasFocus)
            XCUIRemote.shared.press(.select)
            XCTAssertTrue(unmark.waitForExistence(timeout: 5))
            for _ in 0..<8 where !unmark.hasFocus { XCUIRemote.shared.press(.down) }
            XCTAssertTrue(unmark.hasFocus)
            XCUIRemote.shared.press(.select)
            XCTAssertTrue(mark.waitForExistence(timeout: 5))
        }
        let choose = app.buttons["Choose source"]
        XCTAssertTrue(choose.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Owned film without episode metadata"].exists)
        XCTAssertFalse(app.buttons["Season 1"].exists)
        for _ in 0..<8 where !choose.hasFocus { XCUIRemote.shared.press(.down) }
        XCTAssertTrue(choose.hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.buttons["Owned film peer fixture"].waitForExistence(timeout: 5))
        XCUIRemote.shared.press(.menu)
        XCTAssertTrue(choose.waitForExistence(timeout: 5))
        XCTAssertTrue(choose.hasFocus)
        XCUIRemote.shared.press(.menu)
        XCTAssertTrue(title.waitForExistence(timeout: 5))
        XCTAssertTrue(title.hasFocus)
        XCUIRemote.shared.press(.menu)
        XCTAssertTrue(catalog.waitForExistence(timeout: 5))
        XCTAssertTrue(catalog.hasFocus)
    }
    @MainActor func testSearchOpensMatchingTitleAndPreservesQueryOnBack() throws { try searchJourney(empty: false) }
    @MainActor func testEmptySearchExplainsResultAndReturnsToQuery() throws { try searchJourney(empty: true) }
    @MainActor func testFailedSearchRetriesWithoutLosingQuery() throws { try searchJourney(empty: false, retry: true) }
    @MainActor private func searchJourney(empty: Bool, retry: Bool = false) throws {
        let query = retry ? "retry" : (empty ? "missing" : "needle")
        let app = XCUIApplication()
        app.launchEnvironment["CUE_JOURNEY_FIXTURE"] = "1"
        app.launch()
        let search = app.textFields["Search titles in a catalog"]
        XCTAssertTrue(search.waitForExistence(timeout: 20))
        for _ in 0..<8 where !search.hasFocus { XCUIRemote.shared.press(.down) }
        XCTAssertTrue(search.hasFocus)
        XCUIRemote.shared.press(.select)
        search.typeText(query)
        XCUIRemote.shared.press(.menu)
        let catalog = app.buttons["Owned series · series"]
        XCTAssertTrue(catalog.exists)
        XCTAssertFalse(app.buttons["Owned films · movie"].isEnabled, "A catalog without search must not accept a query")
        for _ in 0..<8 where !catalog.hasFocus { XCUIRemote.shared.press(.down) }
        XCTAssertTrue(catalog.hasFocus)
        XCUIRemote.shared.press(.select)
        if retry {
            let retryButton = app.buttons["Retry titles"]
            XCTAssertTrue(retryButton.waitForExistence(timeout: 5))
            XCTAssertTrue(app.staticTexts["Titles could not be loaded. Try again."].exists)
            XCTAssertFalse(app.staticTexts["No titles found. Go back to change your search or choose another catalog."].exists)
            for _ in 0..<8 where !retryButton.hasFocus { XCUIRemote.shared.press(.down) }
            XCTAssertTrue(retryButton.hasFocus)
            XCUIRemote.shared.press(.select)
        }
        if empty {
            XCTAssertTrue(app.staticTexts["No titles found. Go back to change your search or choose another catalog."].waitForExistence(timeout: 5))
            XCTAssertFalse(app.buttons["Owned series 1"].exists)
            XCUIRemote.shared.press(.menu)
            XCTAssertTrue(catalog.waitForExistence(timeout: 5))
            XCTAssertTrue(catalog.hasFocus)
            XCTAssertEqual(search.value as? String, query)
            return
        }
        let result = app.buttons["Needle search result"]
        XCTAssertTrue(result.waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Owned series 1"].exists, "Search must use the query endpoint, not the browse catalog")
        for _ in 0..<8 where !result.hasFocus { XCUIRemote.shared.press(.down) }
        XCTAssertTrue(result.hasFocus)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.buttons["S2 · E2 · Episode 2"].waitForExistence(timeout: 5))
        XCUIRemote.shared.press(.menu)
        XCTAssertTrue(result.waitForExistence(timeout: 5))
        XCTAssertTrue(result.hasFocus)
        XCUIRemote.shared.press(.menu)
        XCTAssertTrue(catalog.waitForExistence(timeout: 5))
        XCTAssertTrue(catalog.hasFocus)
        XCTAssertEqual(search.value as? String, query)
    }
    @MainActor func testEpisodeWatchedControlChangesOnlySelectedEpisodeAndPreservesBackFocus() throws { try journey(play: false, watched: true) }
    @MainActor private func journey(play: Bool, resume: Bool = false, background: Bool = false, relaunch: Bool = false, abrupt: Bool = false, captions: Bool = false, filterSeason: Bool = false, watched: Bool = false) throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_JOURNEY_FIXTURE"] = "1"
        if relaunch { app.launchEnvironment["CUE_JOURNEY_STORAGE"] = UUID().uuidString }
        if abrupt { app.launchEnvironment["CUE_JOURNEY_ABRUPT_EXIT"] = "1" }
        defer {
            if relaunch {
                app.terminate()
                app.launchEnvironment["CUE_JOURNEY_CLEAR_STORAGE"] = "1"
                app.launch()
                XCTAssertTrue(app.staticTexts["Journey storage cleared"].waitForExistence(timeout: 10))
                app.terminate()
            }
        }
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
        if filterSeason {
            let first = app.buttons["Season 1"]
            let second = app.buttons["Season 2"]
            XCTAssertTrue(second.waitForExistence(timeout: 5))
            let firstEpisode = app.buttons["S1 · E1 · Episode 1"]
            for _ in 0..<8 where !firstEpisode.hasFocus { XCUIRemote.shared.press(.down) }
            XCTAssertTrue(firstEpisode.hasFocus)
            XCUIRemote.shared.press(.up)
            if first.hasFocus { XCUIRemote.shared.press(.right) }
            XCTAssertTrue(second.hasFocus, "Season selection must be reachable with the remote")
            XCUIRemote.shared.press(.select)
            expectation(for: NSPredicate { _, _ in !app.buttons["S1 · E1 · Episode 1"].exists }, evaluatedWith: nil)
            waitForExpectations(timeout: 5)
            XCTAssertTrue(app.buttons["S2 · E1 · Episode 1"].exists)
        }
        let episode = app.buttons["S2 · E2 · Episode 2"]
        XCTAssertTrue(episode.waitForExistence(timeout: 5))
        for _ in 0..<12 where !episode.hasFocus { XCUIRemote.shared.press(.down) }
        XCTAssertTrue(episode.hasFocus)
        if watched {
            XCUIRemote.shared.press(.right)
            let mark = app.buttons["Mark as watched: S2 E2"]
            XCTAssertTrue(mark.hasFocus)
            XCUIRemote.shared.press(.select)
            XCTAssertTrue(app.buttons["Mark as unwatched: S2 E2"].waitForExistence(timeout: 5))
            XCTAssertTrue(app.buttons["Mark as watched: S2 E1"].exists)
            XCTAssertTrue(app.buttons["Add to library"].exists)
            XCUIRemote.shared.press(.left)
            XCTAssertTrue(episode.hasFocus)
        }
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
            expectation(for: NSPredicate { _, _ in clock.exists && (Int(clock.label.dropLast()) ?? 0) >= (abrupt ? 20 : (resume ? 8 : 3)) }, evaluatedWith: nil)
            waitForExpectations(timeout: abrupt ? 30 : 20)
            XCTAssertFalse(app.staticTexts["source-playback-error"].exists)
            XCTAssertEqual(clock.value as? String, "Duration: 36 seconds", "Duration must come from the selected decoded fixture")
            let diagnostics = app.descendants(matching: .any).matching(identifier: "source-diagnostics").firstMatch
            XCTAssertTrue(diagnostics.waitForExistence(timeout: 5))
            expectation(for: NSPredicate { _, _ in diagnostics.label.contains("Decoder buffer readiness: 100%") }, evaluatedWith: nil)
            waitForExpectations(timeout: 5)
            XCTAssertTrue(diagnostics.label.contains("Cache allocated:"))
            XCTAssertTrue(diagnostics.label.contains("Verified:"))
            XCTAssertFalse(diagnostics.label.contains("Unknown"))
            for privateValue in ["http", "cue-owned", "urn:btih", "episode02"] {
                XCTAssertFalse(diagnostics.label.contains(privateValue), "Diagnostic presentation must omit source identifiers")
            }
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
            let lastVisibleSeconds = try XCTUnwrap(Int(clock.label.dropLast()))
            if abrupt {
                let namespace = try XCTUnwrap(app.launchEnvironment["CUE_JOURNEY_STORAGE"])
                CFNotificationCenterPostNotification(CFNotificationCenterGetDarwinNotifyCenter(),
                    CFNotificationName("cue.journey.kill.\(namespace)" as CFString), nil, nil, true)
                XCTAssertTrue(app.wait(for: .notRunning, timeout: 5), "SIGKILL must end playback without Close, Home or lifecycle cleanup")
            } else if background {
                XCUIRemote.shared.press(.home)
                expectation(for: NSPredicate { _, _ in
                    app.state == .runningBackground || app.state == .runningBackgroundSuspended
                }, evaluatedWith: nil)
                waitForExpectations(timeout: 5)
                XCTAssertNotEqual(app.state, .notRunning, "Home must not crash or terminate the app")
                app.activate()
                XCTAssertTrue(app.wait(for: .runningForeground, timeout: 10))
                XCTAssertFalse(app.otherElements["vlc-video-surface"].exists, "Returning from Home must not restart playback automatically")
            } else {
                XCUIRemote.shared.press(captions ? .menu : .select)
            }
            if !abrupt {
                XCTAssertTrue(source.waitForExistence(timeout: 5))
                expectation(for: NSPredicate { _, _ in source.hasFocus }, evaluatedWith: nil)
                waitForExpectations(timeout: 5)
                XCTAssertTrue(source.hasFocus, "Closing playback restores the chosen source")
            }
            if resume {
                if relaunch {
                    if !abrupt { app.terminate() }
                    app.launchEnvironment.removeValue(forKey: "CUE_JOURNEY_ABRUPT_EXIT")
                    app.launch()
                    for (index, button) in [catalog, title, episode, source].enumerated() {
                        XCTAssertTrue(button.waitForExistence(timeout: 20))
                        for _ in 0..<20 where !button.hasFocus { XCUIRemote.shared.press(.down) }
                        XCTAssertTrue(button.hasFocus)
                        if index < 3 { XCUIRemote.shared.press(.select) }
                    }
                }
                XCUIRemote.shared.press(.select)
                XCTAssertTrue(app.buttons["source-resume"].waitForExistence(timeout: 10))
                XCTAssertTrue(app.buttons["source-restart"].exists)
                let resumeButton = app.buttons["source-resume"]
                let savedLabel = resumeButton.label
                let savedSeconds = try XCTUnwrap(Int(savedLabel.replacingOccurrences(of: "Resume from ", with: "").dropLast()))
                XCTAssertGreaterThanOrEqual(savedSeconds, 8)
                XCTAssertLessThanOrEqual(abs(savedSeconds - lastVisibleSeconds), 15, "Restored position meets the resume tolerance")
                XCTAssertFalse(app.otherElements["vlc-video-surface"].exists, "Resume choice precedes decoder startup")
                XCUIRemote.shared.press(.menu)
                XCTAssertTrue(source.waitForExistence(timeout: 5))
                expectation(for: NSPredicate { _, _ in source.hasFocus }, evaluatedWith: nil)
                waitForExpectations(timeout: 5)
                XCUIRemote.shared.press(.select)
                XCTAssertTrue(resumeButton.waitForExistence(timeout: 10))
                XCTAssertEqual(resumeButton.label, savedLabel, "Cancelling Resume must not overwrite the saved clock")
                for _ in 0..<3 where !resumeButton.hasFocus { XCUIRemote.shared.press(.up) }
                XCTAssertTrue(resumeButton.hasFocus)
                XCUIRemote.shared.press(.select)
                XCTAssertTrue(clock.waitForExistence(timeout: 25))
                expectation(for: NSPredicate { _, _ in clock.exists && (Int(clock.label.dropLast()) ?? 0) >= savedSeconds - 1 }, evaluatedWith: nil)
                waitForExpectations(timeout: 5)
                if relaunch {
                    let resumedSeconds = try XCTUnwrap(Int(clock.label.dropLast()))
                    print("CUE_RESUME_MEASUREMENT last=\(lastVisibleSeconds) restored=\(savedSeconds) resumed=\(resumedSeconds)")
                }
                XCTAssertFalse(app.staticTexts["source-accounting-error"].exists)
                XCUIRemote.shared.press(.menu)
                XCTAssertTrue(source.waitForExistence(timeout: 5))
                expectation(for: NSPredicate { _, _ in source.hasFocus }, evaluatedWith: nil)
                waitForExpectations(timeout: 5)
                XCUIRemote.shared.press(.select)
                let restart = app.buttons["source-restart"]
                XCTAssertTrue(restart.waitForExistence(timeout: 10))
                for _ in 0..<3 where !restart.hasFocus { XCUIRemote.shared.press(.up) }
                XCTAssertTrue(restart.hasFocus)
                XCUIRemote.shared.press(.select)
                XCTAssertTrue(clock.waitForExistence(timeout: 25))
                expectation(for: NSPredicate { _, _ in clock.exists && (Int(clock.label.dropLast()) ?? 0) >= 1 }, evaluatedWith: nil)
                waitForExpectations(timeout: 5)
                XCTAssertLessThan(try XCTUnwrap(Int(clock.label.dropLast())), 5, "Restart must decode from the beginning, not the saved position")
                XCTAssertFalse(app.staticTexts["source-accounting-error"].exists)
                XCUIRemote.shared.press(.menu)
                XCTAssertTrue(source.waitForExistence(timeout: 5))
                expectation(for: NSPredicate { _, _ in source.hasFocus }, evaluatedWith: nil)
                waitForExpectations(timeout: 5)
            }
        }
        XCUIRemote.shared.press(.menu)
        XCTAssertTrue(episode.waitForExistence(timeout: 5))
        XCTAssertTrue(episode.hasFocus, "Back must restore the selected stable episode identity")
        if watched {
            let unmark = app.buttons["Mark as unwatched: S2 E2"]
            XCTAssertTrue(unmark.waitForExistence(timeout: 5))
            XCUIRemote.shared.press(.right)
            XCTAssertTrue(unmark.hasFocus)
            XCUIRemote.shared.press(.select)
            XCTAssertTrue(app.buttons["Mark as watched: S2 E2"].waitForExistence(timeout: 5))
            XCTAssertTrue(app.buttons["Mark as watched: S2 E1"].exists)
            XCUIRemote.shared.press(.left)
            XCTAssertTrue(episode.hasFocus)
        }
        if filterSeason { XCTAssertFalse(app.buttons["S1 · E1 · Episode 1"].exists, "Back preserves the season filter") }
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
