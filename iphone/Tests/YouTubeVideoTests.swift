import Testing
import Foundation
import Security
@testable import CouchRemote

struct YouTubeVideoTests {
    @Test func supportedLinksKeepOnlyTheVideoID() throws {
        for link in ["https://www.youtube.com/watch?v=jNQXAC9IVRw&si=tracking",
                     "https://m.youtube.com/watch?v=jNQXAC9IVRw&list=private-list",
                     "https://youtu.be/jNQXAC9IVRw?t=12", "https://youtube.com/shorts/jNQXAC9IVRw",
                     "https://www.youtube.com/live/jNQXAC9IVRw", "https://youtube.com/embed/jNQXAC9IVRw"] {
            let url = try #require(URL(string: link))
            #expect(YouTubeVideo(url: url)?.id == "jNQXAC9IVRw")
        }
    }
    @Test func homeAndChannelPagesRequireSelectingAVideo() throws {
        for link in ["https://www.youtube.com/", "https://m.youtube.com/feed/subscriptions", "https://youtube.com/@example", "https://youtube.com/playlist?list=PL123"] {
            #expect(YouTubeVideo(url: try #require(URL(string: link))) == nil)
        }
    }
    @Test func otherSitesAndCredentialBearingURLsAreRejected() throws {
        for link in ["https://youtube.com.example.org/watch?v=jNQXAC9IVRw", "https://notyoutube.com/watch?v=jNQXAC9IVRw",
                     "https://youtube.com@evil.example/watch?v=jNQXAC9IVRw", "https://name:password@youtube.com/watch?v=jNQXAC9IVRw",
                     "http://youtube.com/watch?v=jNQXAC9IVRw", "https://youtube.com:5400/watch?v=jNQXAC9IVRw"] {
            #expect(YouTubeVideo(url: try #require(URL(string: link))) == nil)
        }
    }
    @Test func duplicateIDsAndMalformedVideoIDsAreRejected() throws {
        for link in ["https://youtube.com/watch?v=jNQXAC9IVRw&v=abcdefghijk", "https://youtube.com/watch?v=short",
                     "https://youtu.be/jNQXAC9IVRw/more", "https://youtube.com/watch?v=abcdefghij%3B", "https://youtube.com/watch"] {
            #expect(YouTubeVideo(url: try #require(URL(string: link))) == nil)
        }
    }
    @Test func playbackSuccessNeedsAnExplicitTVAcknowledgement() throws {
        try YouTubePlaybackClient.validateReply(Data(#"{"ok":true}"#.utf8), status: 200)
        for status in [0, 301, 401, 403, 500] {
            #expect(throws: (any Error).self) { try YouTubePlaybackClient.validateReply(Data(#"{"ok":true}"#.utf8), status: status) }
        }
        for body in ["", "<html>not an API response</html>", #"{"ok":false}"#, #"{"unrelated":true}"#] {
            #expect(throws: (any Error).self) { try YouTubePlaybackClient.validateReply(Data(body.utf8), status: 200) }
        }
    }
    @Test func pairingAndTVTimeoutFailuresStayVisible() {
        #expect(throws: TVRemoteError.self) { try YouTubePlaybackClient.validateReply(Data(), status: 401) }
        do {
            try YouTubePlaybackClient.validateReply(Data(#"{"ok":false,"error":"Apple TV did not receive the title."}"#.utf8), status: 504)
            Issue.record("Timeout was treated as success")
        } catch { #expect(error.localizedDescription == "Apple TV did not receive the title.") }
    }
    @Test @MainActor func previousDefaultKeychainPairingIsStillReadable() throws {
        let target = TVTarget(name: "Disposable test TV", host: "10.2.3.4", macAddress: "02:00:00:12:34:56")
        let token = String(repeating: "a", count: 64)
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "com.rupertlaycock.couchtv.remote.pairing", kSecAttrAccount as String: target.credentialID]
        SecItemDelete(query as CFDictionary)
        defer { SecItemDelete(query as CFDictionary) }
        var item = query; item[kSecValueData as String] = Data(token.utf8)
        #expect(SecItemAdd(item as CFDictionary, nil) == errSecSuccess)
        #expect(PairingStore().token(for: target) == token)
    }
}
