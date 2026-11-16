import XCTest

nonisolated final class WatchNavigationTests: XCTestCase {
    @MainActor func testDetailsBackAndSidebarFocus() async throws {
        let app = XCUIApplication()
        app.launchEnvironment["COUCHTV_PREVIEW_URL"] = "http://127.0.0.1:8756"
        app.launch()
        let details = app.buttons["View Details"]
        XCTAssertTrue(details.waitForExistence(timeout: 20))
        for _ in 0..<4 { if details.hasFocus { break }; XCUIRemote.shared.press(.right) }
        XCTAssertTrue(details.hasFocus, "The hero must be reachable from the native sidebar")
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.buttons["Open in Stremio"].waitForExistence(timeout: 5))
        let detail = XCTAttachment(screenshot: app.screenshot()); detail.name = "Native title details"; detail.lifetime = .keepAlways; add(detail)
        XCUIRemote.shared.press(.menu)
        XCTAssertTrue(details.waitForExistence(timeout: 5))
        XCTAssertTrue(details.hasFocus, "Back should restore the focused hero action")
        XCUIRemote.shared.press(.left)
        let tree = XCTAttachment(string: app.debugDescription); tree.name = "Native sidebar accessibility"; tree.lifetime = .keepAlways; add(tree)
        let home = XCTAttachment(screenshot: app.screenshot()); home.name = "Native sidebar"; home.lifetime = .keepAlways; add(home)
        XCUIRemote.shared.press(.down)
        XCUIRemote.shared.press(.select)
        XCTAssertTrue(app.textFields["Films and TV shows"].waitForExistence(timeout: 5))
        var request = URLRequest(url: URL(string: "http://127.0.0.1:8756/api/key")!)
        request.httpMethod = "POST"; request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = Data(#"{"key":"home"}"#.utf8)
        _ = try await URLSession.shared.data(for: request)
        XCTAssertTrue(details.waitForExistence(timeout: 5), "The phone's Home signal should reset Search to Watch Now")
        var inject = URLRequest(url: URL(string: "http://127.0.0.1:8756/__fixture")!)
        inject.httpMethod = "POST"; inject.setValue("application/json", forHTTPHeaderField: "Content-Type")
        inject.httpBody = Data(#"{"clear_events":true,"open_request":{"serial":123,"type":"series","id":"tt1234"}}"#.utf8)
        _ = try await URLSession.shared.data(for: inject)
        var acknowledged = false
        for _ in 0..<40 {
            let (data, _) = try await URLSession.shared.data(from: URL(string: "http://127.0.0.1:8756/__fixture")!)
            let state = try JSONSerialization.jsonObject(with: data) as? [String: Any]
            let events = state?["events"] as? [[String: Any]] ?? []
            acknowledged = events.contains { event in
                let body = event["body"] as? [String: Any]
                return event["path"] as? String == "/api/platform/apple/opened" && body?["serial"] as? Int == 123 && body?["opened"] as? Bool == false
            }
            if acknowledged { break }
            try await Task.sleep(for: .milliseconds(100))
        }
        XCTAssertTrue(acknowledged, "A simulator without Stremio must report a failed native handoff instead of silently dropping the phone's request")
    }
}
