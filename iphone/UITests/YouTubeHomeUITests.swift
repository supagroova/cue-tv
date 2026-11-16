import XCTest

@MainActor
final class YouTubeHomeUITests: XCTestCase {
    func testNativeHomeHasATVButtonAndRejectsNonVideoPages() {
        let app = XCUIApplication()
        app.launchArguments = ["--youtube-home"]
        app.launch()
        let button = app.buttons["Play on TV"]
        XCTAssertTrue(button.waitForExistence(timeout: 30))
        let home = XCTAttachment(screenshot: app.screenshot()); home.name = "YouTube inside TV"; home.lifetime = .keepAlways; add(home)
        button.tap()
        XCTAssertTrue(app.staticTexts["Choose a YouTube video first, then tap the TV button."].waitForExistence(timeout: 25))
        let action = XCTAttachment(screenshot: app.screenshot()); action.name = "TV button on Home explains selection"; action.lifetime = .keepAlways; add(action)
        app.buttons["Close"].tap()
        XCTAssertTrue(button.waitForExistence(timeout: 10))
    }
    func testVideoHandoffUsesThisPhonesPairing() {
        let app = XCUIApplication()
        app.launchArguments = ["--youtube-home", "--youtube-test-video"]
        app.launch()
        let button = app.buttons["Play on TV"]
        XCTAssertTrue(button.waitForExistence(timeout: 30))
        // Safari's page action must run after navigation leaves its initial blank document.
        let page = app.webViews.firstMatch
        XCTAssertTrue(page.waitForExistence(timeout: 30))
        print("Safari controls before navigation settles: \(app.buttons.allElementsBoundByIndex.map(\.label))")
        let navigationComplete = NSPredicate { _, _ in
            app.buttons.allElementsBoundByIndex.contains { $0.label.localizedCaseInsensitiveContains("reload") }
        }
        expectation(for: navigationComplete, evaluatedWith: app)
        waitForExpectations(timeout: 60)
        button.tap()
        XCTAssertTrue(app.staticTexts["Pair this iPhone with the TV using the code on its screen."].waitForExistence(timeout: 25))
        let action = XCTAttachment(screenshot: app.screenshot()); action.name = "A video requires the phone's own pairing"; action.lifetime = .keepAlways; add(action)
    }
}
