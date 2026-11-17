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

import Network
import CryptoKit

@Test func magnetDiscoverySelectionAndVerifiedBytesThroughSwift() async throws {
    let tracker = try NWListener(using: .tcp, on: .any)
    defer { tracker.cancel() }
    tracker.newConnectionHandler = { connection in
        connection.start(queue: .global())
        connection.receive(minimumIncompleteLength: 1, maximumLength: 8192) { _, _, _, _ in
            var body = Data("d8:intervali60e5:peers6:".utf8)
            body.append(contentsOf: [127, 0, 0, 1, 0xa5, 0xb8]) // Controlled fixture peer, port 42424.
            body.append(Data("e".utf8))
            var response = Data("HTTP/1.1 200 OK\r\nContent-Length: \(body.count)\r\nConnection: close\r\n\r\n".utf8)
            response.append(body)
            connection.send(content: response, completion: .contentProcessed { _ in connection.cancel() })
        }
    }
    let port: UInt16 = try await withCheckedThrowingContinuation { continuation in
        tracker.stateUpdateHandler = { state in
            switch state {
            case .ready:
                tracker.stateUpdateHandler = nil
                continuation.resume(returning: tracker.port!.rawValue)
            case .failed(let error):
                tracker.stateUpdateHandler = nil
                continuation.resume(throwing: error)
            default: break
            }
        }
        tracker.start(queue: .global())
    }
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let directory = root.appendingPathComponent("session")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    let service = NativeMetadataService()
    try await service.start(magnet: "magnet:?xt=urn:btih:1be994f3f99cc80c9c2830b113432065d1551e9d&tr=http://127.0.0.1:\(port)/announce", output: directory, useDHT: false)
    var snapshot = try await service.snapshot()
    for _ in 0..<300 where snapshot.state == .loading {
        try await Task.sleep(for: .milliseconds(50))
        snapshot = try await service.snapshot()
    }
    #expect(snapshot.state == .ready)
    #expect(snapshot.files?.first?.index == 0)
    #expect(snapshot.files?.first?.bytes == 175917)
    #expect(try FileManager.default.contentsOfDirectory(atPath: directory.path).isEmpty)
    let playback = try await service.play(fileIndex: 0, output: directory, cacheRoot: root)
    var state = try await playback.snapshot()
    for _ in 0..<100 where state.state == "starting" {
        try await Task.sleep(for: .milliseconds(50))
        state = try await playback.snapshot()
    }
    let url = try #require(state.url.flatMap(URL.init(string:)))
    let (bytes, response) = try await URLSession.shared.data(from: url)
    #expect((response as? HTTPURLResponse)?.statusCode == 200)
    #expect(bytes.count == 175917)
    #expect(SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined() == "426fc0fa1fc53b758d296cef543ecb9a05326a577e5c6c1fb837b12ad038d3fb")
    try await playback.stop()
    try await service.stop()
    try FileManager.default.removeItem(at: root)
}

@Test @MainActor func sourceModelReportsNativeFailureAndRetiresBeforeCacheReuse() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    let cache = TorrentCacheStore(root: root)
    let model = SourcePlaybackModel(cache: cache)
    let request = CoreTorrentRequest(metaId: "title", videoId: "title:1:2", kind: "series", magnet: "invalid", fileIndex: nil, fileMustInclude: [])
    await model.start(.torrent(request))
    #expect(model.coordinator.state == .failed(.invalidMetadata))
    #expect(model.playbackURL == nil)
    await model.stop()
    #expect(model.coordinator.state == .idle)
    #expect(!model.cleanupFailed)
    let next = try await cache.acquire()
    try await cache.release(next)
}

@Test @MainActor func directHTTPSUsesDecoderLifecycleWithoutTorrentCacheAndRejectsLateCallbacks() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    let model = SourcePlaybackModel(cache: TorrentCacheStore(root: root))
    var decoderStops = 0
    model.stopDecoder = { decoderStops += 1 }
    let first = CoreHTTPSRequest(metaId: "title", videoId: "episode-1", kind: "series", url: "https://example.test/one.mp4?signature=synthetic")
    await model.start(.https(first))
    let old = try #require(model.generation)
    #expect(model.playbackURL?.absoluteString == first.url)
    #expect(model.coordinator.state == .buffering)
    #expect(!FileManager.default.fileExists(atPath: root.path))
    let second = CoreHTTPSRequest(metaId: "title", videoId: "episode-2", kind: "series", url: "https://example.test/two.mkv")
    await model.start(.https(second))
    #expect(decoderStops == 1)
    #expect(model.generation != old)
    model.coordinator.receive(.playing, generation: old)
    #expect(model.coordinator.state == .buffering)
    await model.select(fileIndex: 0)
    await model.showFiles(offset: 0)
    #expect(model.playbackURL?.absoluteString == second.url)
    await model.stop()
    #expect(decoderStops == 2)
    #expect(model.playbackURL == nil)
    #expect(model.coordinator.state == .idle)
    #expect(!FileManager.default.fileExists(atPath: root.path))
}

@Test @MainActor func directHTTPSRejectsUnsafeURLBeforeDecoderAdmission() async throws {
    for url in ["http://example.test/video", "file:///tmp/video", "https://user:secret@example.test/video", "https://example.test/video#fragment"] {
        let model = SourcePlaybackModel()
        await model.start(.https(CoreHTTPSRequest(metaId: "title", videoId: "video", kind: "movie", url: url)))
        #expect(model.playbackURL == nil)
        #expect(model.coordinator.state == .failed(.unsupportedFormat))
        await model.stop()
    }
}
