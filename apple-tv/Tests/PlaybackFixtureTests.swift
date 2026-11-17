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
    await session.start()
    let first = try #require(session.cacheDirectory)
    for _ in 0..<100 where session.url == nil {
        try await Task.sleep(for: .milliseconds(50))
    }
    #expect(session.url != nil)
    #expect(FileManager.default.fileExists(atPath: first.path))
    await session.start()
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

@Test @MainActor func playbackFailureReleasesResourcesAndRemainsVisible() async throws {
    let coordinator = PlaybackSessionCoordinator()
    var releases = 0
    let token = try #require(await coordinator.begin { releases += 1 })
    coordinator.receive(.buffering, generation: token)
    coordinator.receive(.failed(.decoder), generation: token)
    for _ in 0..<100 where releases == 0 { await Task.yield() }
    #expect(releases == 1)
    #expect(coordinator.state == .failed(.decoder))
    coordinator.receive(.playing, generation: token)
    #expect(coordinator.state == .failed(.decoder))
    await coordinator.stop()
    #expect(releases == 1)
    #expect(coordinator.state == .idle)
}

@Test @MainActor func unresolvedPlaybackTimesOutAndReleasesResources() async throws {
    let coordinator = PlaybackSessionCoordinator(wait: { _ in })
    var releases = 0
    _ = try #require(await coordinator.begin { releases += 1 })
    for _ in 0..<100 where releases == 0 { await Task.yield() }
    #expect(coordinator.state == .failed(.invalidMetadata))
    #expect(releases == 1)
}

@Test func cacheRecoveryRemovesAbandonedSessionsButProtectsActiveLease() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    let abandoned = root.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: abandoned, withIntermediateDirectories: true)
    try Data("abandoned".utf8).write(to: abandoned.appendingPathComponent("piece"))
    let store = TorrentCacheStore(root: root)
    let active = try await store.acquire()
    #expect(!FileManager.default.fileExists(atPath: abandoned.path))
    do {
        _ = try await store.acquire()
        Issue.record("must not allocate while another cache lease is active")
    } catch {}
    #expect(FileManager.default.fileExists(atPath: active.path))
    try await store.release(active)
    #expect(!FileManager.default.fileExists(atPath: active.path))
}

@Test func cacheRecoveryDoesNotFollowSessionSymlinks() async throws {
    let base = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: base) }
    let root = base.appendingPathComponent("cache")
    let outside = base.appendingPathComponent("outside")
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try FileManager.default.createDirectory(at: outside, withIntermediateDirectories: true)
    let marker = outside.appendingPathComponent("keep")
    try Data("keep".utf8).write(to: marker)
    try FileManager.default.createSymbolicLink(at: root.appendingPathComponent(UUID().uuidString), withDestinationURL: outside)
    let store = TorrentCacheStore(root: root)
    let lease = try await store.acquire()
    #expect(FileManager.default.fileExists(atPath: marker.path))
    try await store.release(lease)
}

@Test func cacheRecoveryPreservesUnrecognizedFiles() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    let file = root.appendingPathComponent("not-a-session")
    try Data("preserve".utf8).write(to: file)
    let store = TorrentCacheStore(root: root)
    do {
        _ = try await store.acquire()
        Issue.record("unrecognized content must prevent recovery")
    } catch {}
    #expect(try Data(contentsOf: file) == Data("preserve".utf8))
}

@Test func cacheCanReopenAfterOperatingSystemPurge() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    let store = TorrentCacheStore(root: root)
    let first = try await store.acquire()
    try await store.release(first)
    try FileManager.default.removeItem(at: root)
    let second = try await store.acquire()
    #expect(FileManager.default.fileExists(atPath: second.path))
    try await store.release(second)
}

@Test func cacheRejectsRootReplacedBySymlinkBetweenSessions() async throws {
    let base = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: base) }
    let root = base.appendingPathComponent("cache")
    let outside = base.appendingPathComponent("outside")
    let store = TorrentCacheStore(root: root)
    let first = try await store.acquire()
    try await store.release(first)
    try FileManager.default.removeItem(at: root)
    try FileManager.default.createDirectory(at: outside, withIntermediateDirectories: true)
    try FileManager.default.createSymbolicLink(at: root, withDestinationURL: outside)
    do {
        _ = try await store.acquire()
        Issue.record("a replaced cache root must not redirect writes")
    } catch {}
    #expect(try FileManager.default.contentsOfDirectory(atPath: outside.path).isEmpty)
}

@Test func cacheReleaseDoesNotFollowReplacedRoot() async throws {
    let base = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: base) }
    let root = base.appendingPathComponent("cache")
    let backup = base.appendingPathComponent("original")
    let outside = base.appendingPathComponent("outside")
    let store = TorrentCacheStore(root: root)
    let lease = try await store.acquire()
    try FileManager.default.moveItem(at: root, to: backup)
    let foreign = outside.appendingPathComponent(lease.lastPathComponent)
    try FileManager.default.createDirectory(at: foreign, withIntermediateDirectories: true)
    let marker = foreign.appendingPathComponent("keep")
    try Data("keep".utf8).write(to: marker)
    try FileManager.default.createSymbolicLink(at: root, withDestinationURL: outside)
    do {
        try await store.release(lease)
        Issue.record("cleanup must not follow a redirected root")
    } catch {}
    #expect(FileManager.default.fileExists(atPath: marker.path))
    try FileManager.default.removeItem(at: root)
    try FileManager.default.moveItem(at: backup, to: root)
    try await store.release(lease)
}

@Test @MainActor func stalledPlaybackTimesOutAndReleasesResources() async throws {
    let coordinator = PlaybackSessionCoordinator(wait: { _ in })
    var releases = 0
    let token = try #require(await coordinator.begin { releases += 1 })
    coordinator.receive(.playing, generation: token)
    for _ in 0..<100 where releases == 0 { await Task.yield() }
    #expect(coordinator.state == .failed(.read))
    #expect(releases == 1)
}

@Test @MainActor func pausedPlaybackDoesNotRunStallDeadline() async throws {
    let coordinator = PlaybackSessionCoordinator(wait: { _ in })
    var releases = 0
    let token = try #require(await coordinator.begin { releases += 1 })
    coordinator.receive(.playing, generation: token)
    coordinator.receive(.paused, generation: token)
    for _ in 0..<100 { await Task.yield() }
    #expect(coordinator.state == .paused)
    #expect(releases == 0)
    await coordinator.stop()
}

@Test @MainActor func onlyAdvancingCurrentPlaybackRefreshesStallDeadline() async throws {
    var timers: [CheckedContinuation<Void, Never>] = []
    defer { for timer in timers { timer.resume() } }
    let coordinator = PlaybackSessionCoordinator(wait: { duration in
        guard duration == .seconds(30) else { throw CancellationError() }
        await withCheckedContinuation { timers.append($0) }
    })
    let token = try #require(await coordinator.begin {})
    coordinator.receive(.playing, generation: token)
    coordinator.reportProgress(milliseconds: 1000, generation: token)
    for _ in 0..<100 where timers.count < 1 { await Task.yield() }
    try #require(timers.count == 1)
    coordinator.reportProgress(milliseconds: 2000, generation: token)
    for _ in 0..<100 where timers.count < 2 { await Task.yield() }
    try #require(timers.count == 2)
    // A canceled deadline cannot fail the still-progressing session.
    timers.removeFirst().resume()
    for _ in 0..<10 { await Task.yield() }
    #expect(coordinator.state == .playing)
    coordinator.reportProgress(milliseconds: 2000, generation: token)
    coordinator.reportProgress(milliseconds: 1500, generation: token)
    coordinator.reportProgress(milliseconds: 3000, generation: UUID())
    coordinator.receive(.playing, generation: token)
    for _ in 0..<10 { await Task.yield() }
    try #require(timers.count == 1)
    timers.removeFirst().resume()
    for _ in 0..<100 where coordinator.state == .playing { await Task.yield() }
    #expect(coordinator.state == .failed(.read))
    await coordinator.stop()
}
