import Foundation
import Testing
@testable import SittingRoomTV

@Suite struct MetadataServiceTests {
    @Test func invalidSourceReportsFailureAndConcurrentStopsReleaseHandle() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let service = NativeMetadataService()
        try await service.start(magnet: "invalid", output: directory, discoverPeers: false)
        var snapshot = try await service.snapshot()
        for _ in 0..<100 where snapshot.state == .loading {
            try await Task.sleep(for: .milliseconds(10))
            snapshot = try await service.snapshot()
        }
        #expect(snapshot.state == .failed)
        #expect(snapshot.errorCode == "invalid_metadata")
        async let first: Void = service.stop()
        async let second: Void = service.stop()
        try await first
        try await second
        try await service.start(magnet: "invalid", output: directory, discoverPeers: false)
        try await service.stop()
        #expect(try FileManager.default.contentsOfDirectory(atPath: directory.path).isEmpty)
    }
}

import CueTorrent

@Test func adoptedPlaybackHandleJoinsConcurrentStops() async throws {
    let output = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: output) }
    let torrent = try #require(Bundle.main.url(forResource: "single", withExtension: "torrent"))
    let input = try JSONSerialization.data(withJSONObject: [
        "api_version": 1, "torrent_path": torrent.path, "output": output.path,
        "file_index": 0, "initial_peers": []
    ])
    let text = try #require(String(data: input, encoding: .utf8))
    let handle = text.withCString { cue_torrent_start($0) }
    #expect(handle != 0)
    let service = NativePlaybackService(adopting: handle)
    var value = try await service.snapshot()
    for _ in 0..<100 where value.state == "starting" {
        try await Task.sleep(for: .milliseconds(10))
        value = try await service.snapshot()
    }
    #expect(value.state == "ready")
    #expect(value.sessionId == handle)
    async let first: Void = service.stop()
    async let second: Void = service.stop()
    try await first
    try await second
    try await service.stop()
    await #expect(throws: NativePlaybackService.Failure.self) { try await service.snapshot() }
    #expect(cue_torrent_stop_ticket(handle) == 0)
}

@Test func sourceSessionOwnsIdentityAndReleasesCacheAfterConcurrentStop() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    let cache = TorrentCacheStore(root: root)
    let request = CoreTorrentRequest(metaId: "title", videoId: "title:1:2", kind: "series", magnet: "invalid", fileIndex: nil, fileMustInclude: [])
    let session = try await TorrentPlaybackSession.create(request: request, cache: cache)
    #expect(session.identity.videoId == "title:1:2")
    await #expect(throws: TorrentCacheStore.Failure.self) { try await cache.acquire() }
    await #expect(throws: (any Error).self) { try await session.select(fileIndex: 99) }
    async let first: Void = session.stop()
    async let second: Void = session.stop()
    try await first
    try await second
    await #expect(throws: TorrentPlaybackSession.Failure.self) { try await session.files() }
    let next = try await cache.acquire()
    #expect(try FileManager.default.contentsOfDirectory(atPath: root.path).count == 1)
    try await cache.release(next)
}
