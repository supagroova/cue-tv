import Foundation
import Observation

/// UI-facing source flow; decoder callbacks use the exposed generation token.
@MainActor @Observable
final class SourcePlaybackModel {
    let coordinator = PlaybackSessionCoordinator()
    var stopDecoder: @MainActor () -> Void = {}
    private(set) var diagnostics: NativePlaybackSnapshot.Diagnostics?
    private(set) var files: TorrentMetadataSnapshot?
    private(set) var playbackURL: URL?
    private(set) var generation: UUID?
    private(set) var cleanupFailed = false
    private var owner: SourceOwner?
    private let cache: TorrentCacheStore
    init(cache: TorrentCacheStore = .shared) { self.cache = cache }

    func start(_ source: CorePlaybackRequest) async {
        let next = SourceOwner()
        guard let token = await coordinator.begin(teardown: { [weak self] in
            self?.stopDecoder()
            do { try await next.stop() }
            catch { self?.cleanupFailed = true }
        }) else { return }
        owner = next; generation = token; files = nil; playbackURL = nil; diagnostics = nil
        guard case .torrent(let request) = source else {
            guard case .https(let request) = source,
                  request.url.utf8.count <= 16384,
                  let url = URL(string: request.url), url.scheme == "https",
                  let host = url.host, !host.isEmpty, url.port != 0,
                  url.user == nil, url.password == nil, url.fragment == nil else {
                coordinator.receive(.failed(.unsupportedFormat), generation: token)
                return
            }
            playbackURL = url
            coordinator.receive(.buffering, generation: token)
            return
        }
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
        guard let token = generation, let owner, !owner.closed, let creation = owner.creation else { return }
        do {
            let session = try await creation.value
            let snapshot = try await session.files(offset: offset)
            guard generation == token, !owner.closed else { return }
            files = snapshot
        } catch {
            coordinator.receive(.failed(.invalidMetadata), generation: token)
        }
    }

    func select(fileIndex: Int) async {
        guard let token = generation, let owner, !owner.closed, let creation = owner.creation else { return }
        coordinator.receive(.buffering, generation: token)
        do {
            let session = try await creation.value
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
                    diagnostics = snapshot.diagnostics
                    owner.monitor = Task { [weak self, weak owner] in
                        do {
                            while !Task.isCancelled {
                                try await Task.sleep(for: .seconds(1))
                                let latest = try await session.snapshot()
                                guard let self, let owner, !owner.closed, self.generation == token else { return }
                                self.diagnostics = latest.diagnostics
                                if latest.state == "failed" {
                                    self.coordinator.receive(.failed(.native(code: latest.errorCode)), generation: token)
                                    return
                                }
                            }
                        } catch {
                            guard !Task.isCancelled, let self, let owner, !owner.closed, self.generation == token else { return }
                            self.diagnostics = nil
                            self.coordinator.receive(.failed(.read), generation: token)
                        }
                    }
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
        generation = nil; playbackURL = nil; files = nil; diagnostics = nil
        await coordinator.stop()
        if owner === previous { owner = nil }
    }
}

@MainActor private final class SourceOwner {
    var creation: Task<TorrentPlaybackSession, any Error>?
    var closed = false
    var monitor: Task<Void, Never>?
    func stop() async throws {
        closed = true
        monitor?.cancel()
        await monitor?.value
        monitor = nil
        guard let creation else { return }
        creation.cancel()
        guard let session = try? await creation.value else { return }
        try await session.stop()
    }
}
