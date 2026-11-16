import Foundation
import Testing
@testable import CouchRemote

struct YouTubeNavigationTests {
    @Test @MainActor func readyRemoteDoesNotReloadWhenAccountWindowReturns() async {
        let model = RemoteModel()
        model.pageLoaded()
        let revision = model.loadRevision
        await model.connect()
        #expect(model.pageReady)
        #expect(!model.connecting)
        #expect(model.loadRevision == revision)
    }
    @Test func accountBridgeAndConsentStayInTheFeedSession() throws {
        for value in ["https://accounts.youtube.com/accounts/SetSID", "https://accounts.google.com/ServiceLogin", "https://consent.youtube.com/", "https://www.google.com/", "https://m.youtube.com/feed/you"] {
            #expect(YouTubeNavigation.isService(try #require(URL(string: value))))
        }
    }
    @Test func lookalikeAndCredentialBearingLinksAreNotAccountNavigation() throws {
        for value in ["https://accounts.youtube.com.example.org/", "https://evilyoutube.com/", "https://google.com.example.org/", "https://accounts.google.com@evil.example/", "https://someone:password@accounts.google.com/", "http://accounts.youtube.com/", "https://accounts.google.com:8443/"] {
            #expect(!YouTubeNavigation.isService(try #require(URL(string: value))))
        }
    }
    @Test func browserSubframesKeepTheirNormalDocumentSchemes() throws {
        for value in ["about:blank", "about:srcdoc", "data:text/html,test", "blob:https://accounts.google.com/example", "https://accounts.youtube.com/accounts/CheckConnection"] {
            #expect(YouTubeNavigation.allowsSubframe(try #require(URL(string: value))))
        }
        for value in ["javascript:alert(1)", "file:///private/example", "intent://example/", "http://example.org/"] {
            #expect(!YouTubeNavigation.allowsSubframe(try #require(URL(string: value))))
        }
    }
}
