import XCTest

/// Opt-in; requires scripts/fixtures/check-account-journey.sh.
nonisolated final class AccountJourneyTests: XCTestCase {
    @MainActor func testRejectedPasswordCanRetryThenBrowseLibraryAndSignOut() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_ACCOUNT_JOURNEY_FIXTURE"] = "1"
        app.launchEnvironment["CUE_ACCOUNT_JOURNEY_STORAGE"] = UUID().uuidString
        app.launch()
        defer {
            app.terminate()
            app.launchEnvironment["CUE_ACCOUNT_JOURNEY_CLEAR"] = "1"
            app.launch()
            XCTAssertTrue(app.staticTexts["Account journey storage cleared"].waitForExistence(timeout: 10))
            app.terminate()
        }
        let open = app.buttons["Sign in again"]
        XCTAssertTrue(open.waitForExistence(timeout: 20))
        focus(open, direction: .down)
        XCUIRemote.shared.press(.select)
        let email = app.textFields["account-refresh-email"]
        let password = app.secureTextFields["account-refresh-password"]
        let submit = app.buttons["account-refresh-submit"]
        XCTAssertTrue(email.waitForExistence(timeout: 10))
        focus(email, direction: .up)
        XCUIRemote.shared.press(.select)
        email.typeText("fixture@example.test")
        XCUIRemote.shared.press(.menu)
        enterPassword("wrong-password", field: password, direction: .down)
        focus(submit, direction: .down)
        XCUIRemote.shared.press(.select)
        let error = app.staticTexts["account-refresh-error"]
        XCTAssertTrue(error.waitForExistence(timeout: 10))
        XCTAssertEqual(error.label, "Check your email and password, then try again.")
        XCTAssertFalse(submit.isEnabled, "A rejected attempt must clear the password before another submission")
        enterPassword("replacement-password", field: password, direction: .up)
        focus(submit, direction: .down)
        XCUIRemote.shared.press(.select)
        expectation(for: NSPredicate(format: "exists == false"), evaluatedWith: submit)
        waitForExpectations(timeout: 10)
        XCTAssertTrue(app.staticTexts["Stremio · Account"].exists)
        expectation(for: NSPredicate(format: "exists == false"), evaluatedWith: open)
        waitForExpectations(timeout: 10)
        let library = app.buttons["My library"]
        focus(library, direction: .down)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.buttons["Owned account title"].waitForExistence(timeout: 10))
        let screenshot = XCTAttachment(screenshot: app.screenshot())
        screenshot.name = "Recovered account library"
        screenshot.lifetime = .keepAlways
        add(screenshot)
        XCUIRemote.shared.press(.menu)
        let signOut = app.buttons["Sign out of Stremio"]
        focus(signOut, direction: .up)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.staticTexts["Stremio · Guest"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.buttons["Sign in to Stremio"].exists)
    }

    @MainActor private func focus(_ element: XCUIElement, direction: XCUIRemote.Button) {
        XCTAssertTrue(element.waitForExistence(timeout: 5))
        for _ in 0..<16 where !element.hasFocus { XCUIRemote.shared.press(direction) }
        XCTAssertTrue(element.hasFocus)
    }
    @MainActor private func enterPassword(_ value: String, field: XCUIElement, direction: XCUIRemote.Button) {
        focus(field, direction: direction)
        XCUIRemote.shared.press(.select)
        field.typeText(value)
        XCUIRemote.shared.press(.menu)
    }
}
