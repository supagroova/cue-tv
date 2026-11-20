import XCTest

@MainActor final class YouTubeInlineUITests: XCTestCase {
    func testAutomaticAddressChangesDoNotPlayUntilExplicitTVAction() {
        let app = XCUIApplication()
        app.launchEnvironment["COUCHTV_REMOTE_PREVIEW"] = "1"
        app.launchEnvironment["COUCHTV_YOUTUBE_SELECTION_PREVIEW"] = "1"
        app.launch()
        let remote = app.webViews["tv-remote-web"]
        XCTAssertTrue(remote.buttons["Watch"].waitForExistence(timeout: 20)); remote.buttons["Watch"].tap()
        let youtube = remote.descendants(matching: .any).matching(NSPredicate(format: "label == %@", "YouTube")).firstMatch
        XCTAssertTrue(youtube.waitForExistence(timeout: 10)); youtube.tap()
        let feed = app.webViews["youtube-inline-web"]
        XCTAssertTrue(feed.staticTexts["Test recommendations"].waitForExistence(timeout: 15))
        let play = app.buttons["Play on TV"]
        XCTAssertTrue(play.waitForExistence(timeout: 10))
        XCTAssertFalse(app.buttons["Try again"].exists)
        XCTAssertFalse(remote.buttons["Remote"].isSelected)
        let screenshot = XCTAttachment(screenshot: app.screenshot()); screenshot.name = "Explicit YouTube TV playback"; screenshot.lifetime = .keepAlways; add(screenshot)
        play.tap()
        // A preview has no household pairing. This error proves only an explicit tap invokes handoff.
        XCTAssertTrue(app.buttons["Try again"].waitForExistence(timeout: 10))
        let cancel = app.buttons["Cancel selected video"]
        XCTAssertTrue(cancel.exists); cancel.tap()
        XCTAssertTrue(app.buttons["Try again"].waitForNonExistence(timeout: 10))
    }
    func testSignInPopupRetainsItsOpenerAndReturnsToTheSameFeed() {
        let app = XCUIApplication()
        app.launchEnvironment["COUCHTV_REMOTE_PREVIEW"] = "1"
        app.launchEnvironment["COUCHTV_YOUTUBE_POPUP_PREVIEW"] = "1"
        app.launch()
        let remote = app.webViews["tv-remote-web"]
        XCTAssertTrue(remote.buttons["Watch"].waitForExistence(timeout: 20)); remote.buttons["Watch"].tap()
        let youtube = remote.descendants(matching: .any).matching(NSPredicate(format: "label == %@", "YouTube")).firstMatch
        XCTAssertTrue(youtube.waitForExistence(timeout: 10)); youtube.tap()
        let feed = app.webViews["youtube-inline-web"]
        let open = feed.buttons["Open test sign-in"]
        XCTAssertTrue(open.waitForExistence(timeout: 15)); open.tap()
        let popup = app.webViews["youtube-signin-window"]
        XCTAssertTrue(popup.waitForExistence(timeout: 10))
        let complete = popup.buttons["Complete test sign-in"]
        XCTAssertTrue(complete.waitForExistence(timeout: 10))
        let image = XCTAttachment(screenshot: app.screenshot()); image.name = "Full-screen account window"; image.lifetime = .keepAlways; add(image)
        // WebKit's about:blank/document.write fixture reports no accessibility hit point.
        // Send a real touch at the visible button, then prove the opener received its response.
        let frame = complete.frame
        XCTAssertTrue(app.frame.contains(frame))
        app.coordinate(withNormalizedOffset: .zero).withOffset(CGVector(dx: frame.midX, dy: frame.midY)).tap()
        XCTAssertTrue(popup.waitForNonExistence(timeout: 10))
        XCTAssertTrue(feed.staticTexts["Test sign-in returned"].waitForExistence(timeout: 10))
        XCTAssertFalse(app.buttons["Back to YouTube"].exists)
    }
    func testFeedManagementAddsRenamesAndRemovesOnlyItsOwnFeed() {
        let app = XCUIApplication()
        app.launchEnvironment["COUCHTV_REMOTE_PREVIEW"] = "1"
        app.launch()
        let remote = app.webViews["tv-remote-web"]
        XCTAssertTrue(remote.buttons["Watch"].waitForExistence(timeout: 20)); remote.buttons["Watch"].tap()
        let youtube = remote.descendants(matching: .any).matching(NSPredicate(format: "label == %@", "YouTube")).firstMatch
        XCTAssertTrue(youtube.waitForExistence(timeout: 10)); youtube.tap()
        func settings() {
            let button = remote.descendants(matching: .any).matching(NSPredicate(format: "label == %@", "YouTube settings")).firstMatch
            XCTAssertTrue(button.waitForExistence(timeout: 10)); button.tap()
            XCTAssertTrue(remote.buttons["Manage YouTube feeds"].waitForExistence(timeout: 10)); remote.buttons["Manage YouTube feeds"].tap()
            XCTAssertTrue(app.navigationBars["YouTube feeds"].waitForExistence(timeout: 10))
        }
        let original = "Ursula QA " + UUID().uuidString.prefix(4)
        let renamed = original + " family"
        settings()
        app.buttons["Add feed"].tap()
        let addName = app.textFields["Feed name"]
        XCTAssertTrue(addName.waitForExistence(timeout: 10))
        addName.tap(); addName.typeText(original); app.navigationBars["Add YouTube feed"].buttons["Add"].tap()
        XCTAssertTrue(app.navigationBars["YouTube feeds"].waitForNonExistence(timeout: 10))
        let selector = remote.descendants(matching: .any).matching(NSPredicate(format: "label == %@", "YouTube feed")).firstMatch
        XCTAssertTrue(selector.waitForExistence(timeout: 10)); XCTAssertTrue((selector.value as? String ?? "").contains(original))
        settings()
        let row = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", original)).firstMatch
        XCTAssertTrue(row.waitForExistence(timeout: 10)); row.tap()
        let field = app.textFields["Feed name"]
        XCTAssertTrue(field.waitForExistence(timeout: 10)); field.tap()
        field.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: original.count) + renamed)
        app.navigationBars["Edit feed"].buttons["Save"].tap()
        let renamedRow = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", renamed)).firstMatch
        XCTAssertTrue(renamedRow.waitForExistence(timeout: 10))
        let image = XCTAttachment(screenshot: app.screenshot()); image.name = "Multiple YouTube feeds on one iPhone"; image.lifetime = .keepAlways; add(image)
        renamedRow.tap(); app.buttons["Remove feed"].tap()
        let confirm = app.popovers.buttons["Remove feed"]
        XCTAssertTrue(confirm.waitForExistence(timeout: 10)); confirm.tap()
        XCTAssertTrue(renamedRow.waitForNonExistence(timeout: 10))
        XCTAssertTrue(app.buttons["Add feed"].exists)
        app.buttons["Done"].tap()
    }
    func testInlineHomeAppearsInWatchAndHidesForSearchAndRemote() {
        let app = XCUIApplication()
        app.launchEnvironment["COUCHTV_REMOTE_PREVIEW"] = "1"
        app.launch()
        let remote = app.webViews["tv-remote-web"]
        XCTAssertTrue(remote.buttons["Watch"].waitForExistence(timeout: 20))
        remote.buttons["Watch"].tap()
        let youtube = remote.descendants(matching: .any).matching(NSPredicate(format: "label == %@", "YouTube")).firstMatch
        XCTAssertTrue(youtube.waitForExistence(timeout: 10)); youtube.tap()
        let feed = app.webViews["youtube-inline-web"]
        XCTAssertTrue(feed.waitForExistence(timeout: 25))
        expectation(for: NSPredicate(format: "hittable == true"), evaluatedWith: feed)
        waitForExpectations(timeout: 15)
        let image = XCTAttachment(screenshot: app.screenshot()); image.name = "Inline YouTube in Watch"; image.lifetime = .keepAlways; add(image)
        let search = remote.searchFields["Search YouTube"]
        XCTAssertTrue(search.waitForExistence(timeout: 10)); search.tap(); search.typeText("Melbourne")
        XCTAssertTrue(remote.buttons["Watch Melbourne video"].waitForExistence(timeout: 15))
        expectation(for: NSPredicate(format: "hittable == false"), evaluatedWith: feed)
        waitForExpectations(timeout: 10)
        remote.buttons["Clear YouTube search"].tap()
        expectation(for: NSPredicate(format: "hittable == true"), evaluatedWith: feed)
        waitForExpectations(timeout: 15)
        remote.buttons["Remote"].tap()
        XCTAssertTrue(remote.buttons["Home"].waitForExistence(timeout: 10))
        expectation(for: NSPredicate(format: "hittable == false"), evaluatedWith: feed)
        waitForExpectations(timeout: 10)
    }
}
