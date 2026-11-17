import Foundation
import Testing
@testable import SittingRoomTV

@Test func fixtureRejectsPathsOutsideBundle() throws {
    let json = #"{"id":"sample","title":"Sample","filename":"../private.mp4","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","durationSeconds":12}"#.data(using: .utf8)!
    #expect(throws: (any Error).self) { try JSONDecoder().decode(PlaybackFixture.self, from: json) }
}

@Test func bundledFixtureManifestIsUsable() throws {
    let fixtures = try PlaybackFixture.load(from: Bundle.main)
    #expect(fixtures.count == 3)
    #expect(Set(fixtures.map(\.id)).count == 3)
    for fixture in fixtures {
        #expect(fixture.durationSeconds == 12)
        #expect(fixture.sha256.count == 64)
        #expect(fixture.url(in: Bundle.main) != nil)
    }
}

@Test @MainActor func torrentCleanupDoesNotDeleteReplacementSession() async throws {
    let session = TorrentLabSession()
    session.start()
    let first = try #require(session.cacheDirectory)
    for _ in 0..<100 where session.url == nil {
        try await Task.sleep(for: .milliseconds(50))
    }
    #expect(session.url != nil)
    #expect(FileManager.default.fileExists(atPath: first.path))
    session.start()
    let second = try #require(session.cacheDirectory)
    #expect(second != first)
    for _ in 0..<100 where session.pendingCleanups > 0 || session.url == nil {
        try await Task.sleep(for: .milliseconds(50))
    }
    #expect(session.pendingCleanups == 0)
    #expect(!FileManager.default.fileExists(atPath: first.path))
    #expect(FileManager.default.fileExists(atPath: second.path))
    session.stop()
    for _ in 0..<100 where session.pendingCleanups > 0 {
        try await Task.sleep(for: .milliseconds(50))
    }
    #expect(session.pendingCleanups == 0)
    #expect(!FileManager.default.fileExists(atPath: second.path))
}
