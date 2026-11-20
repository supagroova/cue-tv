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
        XCTAssertTrue(app.buttons["Episodes & Sources"].waitForExistence(timeout: 5))
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
                return event["path"] as? String == "/api/platform/apple/opened" && body?["serial"] as? Int == 123 && body?["opened"] as? Bool == true
            }
            if acknowledged { break }
            try await Task.sleep(for: .milliseconds(100))
        }
        XCTAssertTrue(acknowledged, "A phone title request opens the native Cue TV detail without requiring Stremio")
    }
    @MainActor func testMatroskaPlaybackAndBackPreserveTheSourceChooser() async throws {
        var reset = URLRequest(url: URL(string: "http://127.0.0.1:8756/__fixture")!)
        reset.httpMethod = "POST"; reset.setValue("application/json", forHTTPHeaderField: "Content-Type")
        reset.httpBody = Data(#"{"open_request":null,"home_request":0,"active":"apple","clear_events":true}"#.utf8)
        _ = try await URLSession.shared.data(for: reset)
        let app = XCUIApplication()
        app.launchEnvironment["COUCHTV_PREVIEW_URL"] = "http://127.0.0.1:8756"
        app.launchEnvironment["COUCHTV_PLAYER_PREVIEW"] = "http://127.0.0.1:8756/cue-test.mkv"
        app.launch()
        func select(_ button: XCUIElement, direction: XCUIRemote.Button = .down) {
            XCTAssertTrue(button.waitForExistence(timeout: 20))
            func focused() -> Bool {
                let row = app.cells.containing(.button, identifier: button.label).firstMatch
                return button.hasFocus || (row.exists && row.hasFocus)
            }
            for _ in 0..<18 { if focused() { break }; XCUIRemote.shared.press(direction) }
            if !focused() { let tree = XCTAttachment(string: app.debugDescription); tree.name = "Source focus diagnostic"; tree.lifetime = .keepAlways; add(tree) }
            XCTAssertTrue(focused()); XCUIRemote.shared.press(.select)
        }
        select(app.buttons["View Details"], direction: .right)
        select(app.buttons["Episodes & Sources"])
        let episode = app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "1. A quiet morning")).firstMatch
        select(episode)
        let source = app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "4K HDR")).firstMatch
        select(source)
        XCTAssertTrue(app.staticTexts["Cue TV player test"].waitForExistence(timeout: 15))
        XCTAssertTrue(app.buttons["Pause"].waitForExistence(timeout: 20))
        var reportedProgress = false
        for _ in 0..<75 {
            let (data, _) = try await URLSession.shared.data(from: URL(string: "http://127.0.0.1:8756/__fixture")!)
            let state = try JSONSerialization.jsonObject(with: data) as? [String: Any]
            let events = state?["events"] as? [[String: Any]] ?? []
            reportedProgress = events.contains { event in
                let body = event["body"] as? [String: Any]
                return event["path"] as? String == "/api/stremio/progress" && (body?["position"] as? Int ?? 0) > 0 && (body?["duration"] as? Int ?? 0) > 0
            }
            if reportedProgress { break }; try await Task.sleep(for: .milliseconds(200))
        }
        XCUIRemote.shared.press(.playPause)
        XCTAssertTrue(app.buttons["Play"].waitForExistence(timeout: 5))
        let shot = XCTAttachment(screenshot: app.screenshot()); shot.name = "Matroska playing inside Cue TV"; shot.lifetime = .keepAlways; add(shot)
        XCTAssertTrue(reportedProgress, "The actual VLC player must advance and report media duration")
        XCUIRemote.shared.press(.menu)
        XCTAssertTrue(source.waitForExistence(timeout: 5)); XCTAssertTrue(source.isHittable)
        XCTAssertFalse(app.buttons["Back to browsing"].exists)
        let back = XCTAttachment(screenshot: app.screenshot()); back.name = "Source chooser retained after playback"; back.lifetime = .keepAlways; add(back)
    }
}
