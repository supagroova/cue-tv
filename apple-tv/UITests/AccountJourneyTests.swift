import XCTest

/// Opt-in; requires scripts/fixtures/check-account-journey.sh.
nonisolated final class AccountJourneyTests: XCTestCase {
    @MainActor func testRejectedPasswordCanRetryThenBrowseLibraryAndSignOut() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_ACCOUNT_JOURNEY_FIXTURE"] = "1"
        app.launchEnvironment["CUE_ACCOUNT_JOURNEY_STORAGE"] = UUID().uuidString
        registerCleanup(app)
        app.launch()
        let open = app.buttons["Sign in again"]
        XCTAssertTrue(open.waitForExistence(timeout: 20))
        try focus(open, direction: .down)
        XCUIRemote.shared.press(.select)
        let email = app.textFields["account-refresh-email"]
        let password = app.secureTextFields["account-refresh-password"]
        let submit = app.buttons["account-refresh-submit"]
        XCTAssertTrue(email.waitForExistence(timeout: 10))
        try focus(email, direction: .up)
        XCUIRemote.shared.press(.select)
        email.typeText("fixture@example.test")
        XCUIRemote.shared.press(.menu)
        try enterPassword("wrong-password", field: password, direction: .down)
        try focus(submit, direction: .down)
        XCUIRemote.shared.press(.select)
        let error = app.staticTexts["account-refresh-error"]
        XCTAssertTrue(error.waitForExistence(timeout: 10))
        XCTAssertEqual(error.label, "Check your email and password, then try again.")
        XCTAssertFalse(submit.isEnabled, "A rejected attempt must clear the password before another submission")
        try enterPassword("replacement-password", field: password, direction: .up)
        try focus(submit, direction: .down)
        XCUIRemote.shared.press(.select)
        expectation(for: NSPredicate(format: "exists == false"), evaluatedWith: submit)
        waitForExpectations(timeout: 10)
        XCTAssertTrue(app.staticTexts["Stremio · Account"].exists)
        expectation(for: NSPredicate(format: "exists == false"), evaluatedWith: open)
        waitForExpectations(timeout: 10)
        let library = app.buttons["My library"]
        try focus(library, direction: .down)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.buttons["Owned account title"].waitForExistence(timeout: 10))
        let screenshot = XCTAttachment(screenshot: app.screenshot())
        screenshot.name = "Recovered account library"
        screenshot.lifetime = .keepAlways
        add(screenshot)
        XCUIRemote.shared.press(.menu)
        let signOut = app.buttons["Sign out of Stremio"]
        try focus(signOut, direction: .up)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.staticTexts["Stremio · Guest"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.buttons["Sign in to Stremio"].exists)
        app.launchEnvironment["CUE_ACCOUNT_JOURNEY_EXPECT_EMPTY"] = "1"
    }

    @MainActor func testGuestCanRejectThenAdoptAccountAndSignOut() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_ACCOUNT_JOURNEY_FIXTURE"] = "1"
        app.launchEnvironment["CUE_ACCOUNT_JOURNEY_FIRST_SIGN_IN"] = "1"
        app.launchEnvironment["CUE_ACCOUNT_JOURNEY_STORAGE"] = UUID().uuidString
        registerCleanup(app)
        app.launch()
        let open = app.buttons["Sign in to Stremio"]
        XCTAssertTrue(open.waitForExistence(timeout: 20))
        XCTAssertTrue(app.staticTexts["Stremio · Guest"].exists)
        try focus(open, direction: .down)
        XCUIRemote.shared.press(.select)
        let email = app.textFields["account-signin-email"]
        let password = app.secureTextFields["account-signin-password"]
        let submit = app.buttons["account-signin-submit"]
        XCTAssertTrue(email.waitForExistence(timeout: 10))
        try focus(email, direction: .up)
        XCUIRemote.shared.press(.select)
        email.typeText("fixture@example.test")
        XCUIRemote.shared.press(.menu)
        try enterPassword("wrong-password", field: password, direction: .down)
        try focus(submit, direction: .down)
        XCUIRemote.shared.press(.select)
        let error = app.staticTexts["account-signin-error"]
        XCTAssertTrue(error.waitForExistence(timeout: 10))
        XCTAssertEqual(error.label, "Sign-in failed. Check your details and connection, then try again.")
        XCTAssertFalse(submit.isEnabled)
        try enterPassword("fixture-password", field: password, direction: .up)
        try focus(submit, direction: .down)
        XCUIRemote.shared.press(.select)
        expectation(for: NSPredicate(format: "exists == false"), evaluatedWith: submit)
        waitForExpectations(timeout: 10)
        XCTAssertTrue(app.staticTexts["Stremio · Account"].waitForExistence(timeout: 10))
        let catalog = app.buttons["Owned · movie"]
        try focus(catalog, direction: .down)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.buttons["Owned account title"].waitForExistence(timeout: 10))
        let screenshot = XCTAttachment(screenshot: app.screenshot())
        screenshot.name = "Signed-in account catalog"
        screenshot.lifetime = .keepAlways
        add(screenshot)
        XCUIRemote.shared.press(.menu)
        let signOut = app.buttons["Sign out of Stremio"]
        try focus(signOut, direction: .up)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.staticTexts["Stremio · Guest"].waitForExistence(timeout: 10))
        app.launchEnvironment["CUE_ACCOUNT_JOURNEY_EXPECT_EMPTY"] = "1"
    }

    @MainActor func testUnreadableSelectionRequiresConfirmationAndReturnsToGuest() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CUE_ACCOUNT_JOURNEY_FIXTURE"] = "1"
        app.launchEnvironment["CUE_ACCOUNT_JOURNEY_INVALID_SELECTION"] = "1"
        app.launchEnvironment["CUE_ACCOUNT_JOURNEY_STORAGE"] = UUID().uuidString
        registerCleanup(app)
        app.launch()
        let reset = app.buttons["reset-account-selection"]
        XCTAssertTrue(reset.waitForExistence(timeout: 10))
        XCTAssertFalse(app.buttons["Sign in to Stremio"].exists)
        try focus(reset, direction: .down)
        XCUIRemote.shared.press(.select)
        let confirm = app.buttons["Reset selection"].firstMatch
        XCTAssertTrue(confirm.waitForExistence(timeout: 5))
        let screenshot = XCTAttachment(screenshot: app.screenshot())
        screenshot.name = "Unreadable account selection confirmation"
        screenshot.lifetime = .keepAlways
        add(screenshot)
        try focus(app.buttons["Cancel"].firstMatch, direction: .left)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(reset.waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Sign in to Stremio"].exists)
        try focus(reset, direction: .up)
        XCUIRemote.shared.press(.select)
        try focus(confirm, direction: .right)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.buttons["My library"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.buttons["Sign in to Stremio"].exists)
        XCTAssertFalse(reset.exists)
        app.launchEnvironment["CUE_ACCOUNT_JOURNEY_EXPECT_EMPTY"] = "1"
    }

    private enum NavigationFailure: Error { case missingElement, missingFocus }

    @MainActor private func registerCleanup(_ app: XCUIApplication) {
        addTeardownBlock { @MainActor () async throws in
            app.terminate()
            app.launchEnvironment["CUE_ACCOUNT_JOURNEY_CLEAR"] = "1"
            app.launch()
            XCTAssertTrue(app.staticTexts["Account journey storage cleared"].waitForExistence(timeout: 10))
            app.terminate()
        }
    }
    @MainActor private func focus(_ element: XCUIElement, direction: XCUIRemote.Button) throws {
        guard element.waitForExistence(timeout: 5) else {
            XCTFail("Expected navigation element is missing")
            throw NavigationFailure.missingElement
        }
        for _ in 0..<16 where !element.hasFocus { XCUIRemote.shared.press(direction) }
        guard element.hasFocus else {
            XCTFail("Remote navigation did not focus the expected element")
            throw NavigationFailure.missingFocus
        }
    }
    @MainActor private func enterPassword(_ value: String, field: XCUIElement, direction: XCUIRemote.Button) throws {
        try focus(field, direction: direction)
        XCUIRemote.shared.press(.select)
        field.typeText(value)
        XCUIRemote.shared.press(.menu)
    }
}
