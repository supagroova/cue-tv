import Foundation
import CueTorrent

nonisolated struct TorrentMetadataSnapshot: Decodable, Sendable {
    enum State: String, Decodable, Sendable { case loading, ready, failed, stopping }
    struct File: Decodable, Sendable { let index: Int; let path: String; let bytes: UInt64; let padding: Bool }
    let apiVersion: Int
    let sessionId: UInt64
    let state: State
    let offset: Int?
    let total: Int?
    let files: [File]?
    let errorCode: String?
}

/// Serializes metadata discovery outside the UI actor; paths are display data only.
actor NativeMetadataService {
    enum Failure: Error { case busy, invalidRequest, unavailable, malformedSnapshot, shutdownTimeout }
    private var handle: UInt64?
    private var retirement: Task<Void, any Error>?

    func start(magnet: String, output: URL, discoverPeers: Bool = true) throws {
        guard handle == nil else { throw Failure.busy }
        guard output.isFileURL, magnet.utf8.count <= 16384 else { throw Failure.invalidRequest }
        let input = try JSONSerialization.data(withJSONObject: [
            "api_version": 1, "magnet": magnet, "output": output.path,
            "discovery": ["trackers": discoverPeers, "dht": discoverPeers]
        ])
        let id = input.withUnsafeBytes {
            cue_metadata_start($0.bindMemory(to: UInt8.self).baseAddress, $0.count)
        }
        guard id != 0 else { throw Failure.unavailable }
        handle = id
    }

    func snapshot(offset: Int = 0) throws -> TorrentMetadataSnapshot {
        guard let handle, retirement == nil else { throw Failure.unavailable }
        guard offset >= 0 else { throw Failure.invalidRequest }
        // 24 paths of at most 4096 bytes, allowing JSON escaping and envelope overhead.
        var buffer = [UInt8](repeating: 0, count: 1024 * 1024)
        let count = buffer.withUnsafeMutableBufferPointer {
            cue_metadata_snapshot(handle, offset, $0.baseAddress, $0.count)
        }
        guard count > 0, count <= buffer.count else { throw Failure.malformedSnapshot }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        guard let value = try? decoder.decode(TorrentMetadataSnapshot.self, from: Data(buffer.prefix(count))),
              value.apiVersion == 1, value.sessionId == handle,
              (value.files?.count ?? 0) <= 24 else { throw Failure.malformedSnapshot }
        return value
    }

    func stop() async throws {
        guard let handle else { return }
        let task: Task<Void, any Error>
        if let retirement { task = retirement }
        else {
            task = Task.detached { try await Self.retire(handle) }
            retirement = task
        }
        do {
            try await task.value
            if self.handle == handle { self.handle = nil; retirement = nil }
        } catch {
            if self.handle == handle { retirement = nil }
            throw error
        }
    }

    private nonisolated static func retire(_ handle: UInt64) async throws {
        let deadline = ContinuousClock.now.advanced(by: .seconds(5))
        while cue_metadata_stop(handle) == 0 {
            guard ContinuousClock.now < deadline else { throw Failure.shutdownTimeout }
            try await Task.sleep(for: .milliseconds(10))
        }
        guard cue_metadata_release(handle) == 1 else { throw Failure.unavailable }
    }

    deinit {
        if let handle, retirement == nil {
            Task.detached { try? await Self.retire(handle) }
        }
    }
}
