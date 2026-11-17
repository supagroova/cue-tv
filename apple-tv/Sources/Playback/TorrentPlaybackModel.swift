import Foundation
import Observation

/// UI-facing source flow; decoder callbacks use the exposed generation token.
@MainActor @Observable
final class TorrentPlaybackModel {
    let coordinator = PlaybackSessionCoordinator()
    private(set) var files: TorrentMetadataSnapshot?
    private(set) var playbackURL: URL?
    private(set) var generation: UUID?
    private(set) var cleanupFailed = false
    private var owner: SourceOwner?
    private let cache: TorrentCacheStore
    init(cache: TorrentCacheStore = .shared) { self.cache = cache }

    func start(_ request: CoreTorrentRequest) async {
        let next = SourceOwner()
        guard let token = await coordinator.begin(teardown: { [weak self] in
            do { try await next.stop() }
            catch { self?.cleanupFailed = true }
        }) else { return }
        owner = next; generation = token; files = nil; playbackURL = nil
        let cache = cache
        next.creation = Task { try await TorrentPlaybackSession.create(request: request, cache: cache) }
        coordinator.receive(.discovering, generation: token)
        do {
            let session = try await next.creation!.value
            while generation == token, !next.closed {
                let snapshot = try await session.files()
                guard generation == token, !next.closed else { return }
                switch snapshot.state {
                case .loading: try await Task.sleep(for: .milliseconds(100))
                case .ready:
                    files = snapshot
                    if let index = request.fileIndex { await select(fileIndex: Int(index)) }
                    else { coordinator.receive(.choosingFile, generation: token) }
                    return
                case .failed:
                    coordinator.receive(.failed(.native(code: snapshot.errorCode)), generation: token)
                    return
                case .stopping: return
                }
            }
        } catch {
            if generation == token, !next.closed { coordinator.receive(.failed(.discovery), generation: token) }
        }
    }

    func showFiles(offset: Int) async {
        guard let token = generation, let owner, !owner.closed else { return }
        do {
            let session = try await owner.creation!.value
            let snapshot = try await session.files(offset: offset)
            guard generation == token, !owner.closed else { return }
            files = snapshot
        } catch {
            coordinator.receive(.failed(.invalidMetadata), generation: token)
        }
    }

    func select(fileIndex: Int) async {
        guard let token = generation, let owner, !owner.closed else { return }
        coordinator.receive(.buffering, generation: token)
        do {
            let session = try await owner.creation!.value
            try await session.select(fileIndex: fileIndex)
            while generation == token, !owner.closed {
                let snapshot = try await session.snapshot()
                guard generation == token, !owner.closed else { return }
                if snapshot.state == "failed" {
                    coordinator.receive(.failed(.native(code: snapshot.errorCode)), generation: token)
                    return
                }
                if snapshot.state == "ready", let url = snapshot.url.flatMap(URL.init(string:)),
                   url.scheme == "http", url.host == "127.0.0.1" {
                    playbackURL = url
                    return // Decoder reports playing/progress; bytes alone do not imply playback.
                }
                try await Task.sleep(for: .milliseconds(100))
            }
        } catch {
            if generation == token, !owner.closed { coordinator.receive(.failed(.read), generation: token) }
        }
    }

    func stop() async {
        let previous = owner
        generation = nil; playbackURL = nil; files = nil
        await coordinator.stop()
        if owner === previous { owner = nil }
    }
}

@MainActor private final class SourceOwner {
    var creation: Task<TorrentPlaybackSession, any Error>?
    var closed = false
    func stop() async throws {
        closed = true
        guard let creation else { return }
        creation.cancel()
        guard let session = try? await creation.value else { return }
        try await session.stop()
    }
}
