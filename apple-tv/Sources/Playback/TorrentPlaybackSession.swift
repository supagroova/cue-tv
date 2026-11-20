import Foundation

/// One source owns discovery, selected transfer and the disposable cache lease.
/// The UI coordinator owns this session and awaits stop before starting another.
actor TorrentPlaybackSession {
    enum Failure: Error { case stopped, alreadySelected, unavailable }
    nonisolated let identity: CoreTorrentRequest
    private let metadata: NativeMetadataService
    private let cache: TorrentCacheStore
    private let directory: URL
    private var selection: Task<NativePlaybackService, any Error>?
    private var playback: NativePlaybackService?
    private var retirement: Task<Void, any Error>?
    private var stopped = false

    private init(identity: CoreTorrentRequest, metadata: NativeMetadataService, cache: TorrentCacheStore, directory: URL) {
        self.identity = identity; self.metadata = metadata; self.cache = cache; self.directory = directory
    }

    static func create(request: CoreTorrentRequest, cache: TorrentCacheStore = .shared) async throws -> TorrentPlaybackSession {
        try Task.checkCancellation()
        let directory = try await cache.acquire()
        let metadata = NativeMetadataService()
        do {
            try Task.checkCancellation()
            try await metadata.start(magnet: request.magnet, output: directory)
            try Task.checkCancellation()
            return TorrentPlaybackSession(identity: request, metadata: metadata, cache: cache, directory: directory)
        } catch {
            // Do not release the directory if discovery shutdown is unconfirmed.
            try await metadata.stop()
            try await cache.release(directory)
            throw error
        }
    }

    func files(offset: Int = 0) async throws -> TorrentMetadataSnapshot {
        guard !stopped, retirement == nil, selection == nil else { throw Failure.stopped }
        let snapshot = try await metadata.snapshot(offset: offset)
        guard !stopped, retirement == nil, selection == nil else { throw Failure.stopped }
        return snapshot
    }

    func select(fileIndex: Int) async throws {
        guard !stopped, retirement == nil else { throw Failure.stopped }
        guard selection == nil else { throw Failure.alreadySelected }
        let metadata = metadata, directory = directory
        let task = Task { try await metadata.play(fileIndex: fileIndex, output: directory, cacheRoot: directory.deletingLastPathComponent()) }
        selection = task
        do {
            let service = try await task.value
            // A concurrent stop already owns the task result and its shutdown.
            guard retirement == nil, !stopped else { throw Failure.stopped }
            playback = service
        } catch {
            if retirement == nil, !stopped { selection = nil }
            throw error
        }
    }

    func snapshot() async throws -> NativePlaybackSnapshot {
        guard !stopped, retirement == nil, let playback else { throw Failure.unavailable }
        let value = try await playback.snapshot()
        guard !stopped, retirement == nil else { throw Failure.stopped }
        return value
    }

    func stop() async throws {
        if stopped { return }
        let task: Task<Void, any Error>
        if let retirement { task = retirement }
        else {
            let selection = selection, metadata = metadata, cache = cache, directory = directory
            task = Task.detached {
                if let selection, let playback = try? await selection.value { try await playback.stop() }
                try await metadata.stop()
                try await cache.release(directory)
            }
            retirement = task
        }
        do {
            try await task.value
            stopped = true; selection = nil; playback = nil; retirement = nil
        } catch {
            retirement = nil
            throw error
        }
    }

    deinit {
        if !stopped, retirement == nil {
            let selection = selection, metadata = metadata, cache = cache, directory = directory
            Task.detached {
                do {
                    if let selection, let playback = try? await selection.value { try await playback.stop() }
                    try await metadata.stop()
                    try await cache.release(directory)
                } catch { /* Retain cache if shutdown is unconfirmed. */ }
            }
        }
    }
}
