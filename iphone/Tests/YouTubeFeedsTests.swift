import Foundation
import Testing
import WebKit
@testable import CouchRemote

struct YouTubeFeedsTests {
    @Test func feedListsBelongToEachPhoneAndKeepStableSessionIdentifiers() throws {
        var rupert = YouTubeFeeds.restore(nil)
        let initial = rupert.selectedID
        let renamed = rupert.rename(initial, to: "Rupert"); #expect(renamed)
        let addedUrsula = rupert.add("Ursula")
        let ursula = try #require(addedUrsula)
        #expect(ursula != initial); #expect(rupert.selectedID == ursula)
        let restored = YouTubeFeeds.restore(try JSONEncoder().encode(rupert))
        #expect(restored == rupert)
        var wife = YouTubeFeeds.restore(nil)
        let renamedWife = wife.rename(wife.selectedID, to: "Ursula"); #expect(renamedWife)
        #expect(wife.profiles.count == 1); #expect(wife.selectedID != ursula)
        let switched = rupert.select(initial); #expect(switched); #expect(rupert.profiles.count == 2)
    }
    @Test func renamingNeverChangesTheSignInStoreAndRemovingKeepsAnotherFeed() throws {
        var feeds = YouTubeFeeds.restore(nil)
        let first = feeds.selectedID
        let added = feeds.add("Ursula")
        let second = try #require(added)
        let renamed = feeds.rename(first, to: "Rupert"); #expect(renamed); #expect(feeds.profiles[0].id == first)
        let removed = feeds.remove(second); #expect(removed); #expect(feeds.selectedID == first)
        let removedLast = feeds.remove(first), selectedMissing = feeds.select(second), renamedMissing = feeds.rename(second, to: "Unknown")
        #expect(!removedLast); #expect(!selectedMissing); #expect(!renamedMissing)
    }
    @Test func namesAndProfileCountsAreBounded() {
        var feeds = YouTubeFeeds.restore(nil)
        let blank = feeds.add("  "), long = feeds.add(String(repeating: "x", count: 41))
        #expect(blank == nil); #expect(long == nil)
        let ursula = feeds.add("  Ursula  "); #expect(ursula != nil); #expect(feeds.profiles.last?.name == "Ursula")
        let duplicate = feeds.add("ursula"); #expect(duplicate == nil)
        while feeds.profiles.count < YouTubeFeeds.maximum {
            let added = feeds.add("Feed \(feeds.profiles.count)"); #expect(added != nil)
        }
        let overflow = feeds.add("Too many"); #expect(overflow == nil)
    }
    @Test @MainActor func websiteSignInsStayInSeparatePersistentStores() async throws {
        let first = WKWebsiteDataStore(forIdentifier: UUID())
        let second = WKWebsiteDataStore(forIdentifier: UUID())
        #expect(first.identifier != second.identifier)
        let cookie = try #require(HTTPCookie(properties: [.domain: "feed-fixture.invalid", .path: "/", .name: "synthetic-profile", .value: "first", .secure: "TRUE"]))
        await first.httpCookieStore.setCookie(cookie)
        let containsFirst = await withCheckedContinuation { (continuation: CheckedContinuation<Bool, Never>) in
            first.httpCookieStore.getAllCookies { values in continuation.resume(returning: values.contains { $0.name == "synthetic-profile" && $0.value == "first" }) }
        }
        let containsSecond = await withCheckedContinuation { (continuation: CheckedContinuation<Bool, Never>) in
            second.httpCookieStore.getAllCookies { values in continuation.resume(returning: values.contains { $0.name == "synthetic-profile" }) }
        }
        #expect(containsFirst); #expect(!containsSecond)
        await first.removeData(ofTypes: WKWebsiteDataStore.allWebsiteDataTypes(), modifiedSince: .distantPast)
        await second.removeData(ofTypes: WKWebsiteDataStore.allWebsiteDataTypes(), modifiedSince: .distantPast)
    }
}
