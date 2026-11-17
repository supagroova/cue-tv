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

@Test @MainActor func playbackCoordinatorRejectsRetiredCallbacks() async {
    let coordinator = PlaybackSessionCoordinator()
    var stopped = 0
    let first = await coordinator.begin { stopped += 1 }
    #expect(first != nil)
    coordinator.receive(.playing, generation: first!)
    let second = await coordinator.begin { stopped += 1 }
    #expect(stopped == 1)
    #expect(second != first)
    coordinator.receive(.failed(.decoder), generation: first!)
    #expect(coordinator.state == .resolving)
    coordinator.receive(.buffering, generation: second!)
    coordinator.receive(.playing, generation: second!)
    #expect(coordinator.state == .playing)
    await coordinator.stop()
    coordinator.receive(.playing, generation: second!)
    #expect(coordinator.state == .idle)
    #expect(stopped == 2)
}

@Test @MainActor func playbackCoordinatorSerializesOverlappingSourceSwitches() async throws {
    let coordinator = PlaybackSessionCoordinator()
    var release: CheckedContinuation<Void, Never>?
    _ = await coordinator.begin {
        await withCheckedContinuation { release = $0 }
    }
    let second = Task { await coordinator.begin {} }
    for _ in 0..<100 where release == nil { await Task.yield() }
    let barrier = try #require(release)
    let third = Task { await coordinator.begin {} }
    for _ in 0..<10 { await Task.yield() }
    #expect(coordinator.state == .stopping)
    barrier.resume()
    #expect(await second.value == nil)
    #expect(await third.value != nil)
    #expect(coordinator.state == .resolving)
    await coordinator.stop()
}
