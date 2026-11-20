import XCTest

@MainActor final class RemoteControlsUITests: XCTestCase {
    private let base = URL(string: "http://127.0.0.1:5400/")!
    private func fixture(_ body: [String: Any]) async throws {
        var request = URLRequest(url: base.appendingPathComponent("__fixture"))
        request.httpMethod = "POST"; request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        _ = try await URLSession.shared.data(for: request)
    }
    private func keys() async throws -> [[String: Any]] {
        let (data, _) = try await URLSession.shared.data(from: base.appendingPathComponent("__fixture"))
        let state = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
        return (state["events"] as? [[String: Any]] ?? []).filter { $0["path"] as? String == "/api/key" }.compactMap { $0["body"] as? [String: Any] }
    }
    func testHeldSelectAndTVButtonInWebKit() async throws {
        try await fixture(["active": "apple", "clear_events": true])
        let app = XCUIApplication(); app.launchEnvironment["COUCHTV_REMOTE_PREVIEW"] = "1"; app.launch()
        let remote = app.webViews["tv-remote-web"]
        let tv = remote.buttons["TV button. Hold for Control Centre"]
        XCTAssertTrue(tv.waitForExistence(timeout: 20))
        let pad = remote.otherElements.matching(NSPredicate(format: "label BEGINSWITH %@", "Swipe pad.")).firstMatch
        XCTAssertTrue(pad.exists)
        try await fixture(["clear_events": true])
        pad.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).press(forDuration: 1.2)
        try await Task.sleep(for: .milliseconds(250))
        var sent = try await keys(); XCTAssertEqual(sent.count, 1)
        XCTAssertEqual(sent.first?["key"] as? String, "ok"); XCTAssertEqual(sent.first?["hold"] as? Bool, true)
        XCTAssertFalse(app.menuItems["Copy"].exists)
        try await fixture(["clear_events": true])
        tv.press(forDuration: 1.2)
        try await Task.sleep(for: .milliseconds(250))
        sent = try await keys(); XCTAssertEqual(sent.count, 1)
        XCTAssertEqual(sent.first?["key"] as? String, "apple_tv"); XCTAssertEqual(sent.first?["hold"] as? Bool, true)
        let image = XCTAttachment(screenshot: app.screenshot()); image.name = "Cue TV remote with TV button"; image.lifetime = .keepAlways; add(image)
        try await fixture(["active": "android", "clear_events": true])
    }
}
