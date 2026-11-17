import Foundation
import Observation

@MainActor @Observable
final class PlaybackSubtitlesModel {
    private(set) var url: URL?
    private(set) var selectionID: UUID?
    private(set) var error: String?
    private(set) var loading = false
    private(set) var discovered: [CoreSubtitle] = []
    private(set) var discoveryError: String?
    private var discovery: Task<CoreSubtitleDiscovery, any Error>?
    private var discoveryGeneration = UUID()
    func discover(using action: @escaping @MainActor () async throws -> CoreSubtitleDiscovery) async {
        guard !closed else { return }
        let token = UUID(); discoveryGeneration = token
        let previous = discovery
        previous?.cancel()
        let task = Task {
            _ = await previous?.result
            try Task.checkCancellation()
            return try await action()
        }
        discovery = task; discoveryError = nil
        do {
            let result = try await withTaskCancellationHandler { try await task.value } onCancel: { task.cancel() }
            guard !closed, discoveryGeneration == token, !Task.isCancelled else { return }
            discovered = Array(result.tracks.prefix(32))
            if result.failedProviders > 0 { discoveryError = "Some subtitle providers did not respond." }
        } catch {
            if !closed, discoveryGeneration == token, !Task.isCancelled { discoveryError = "Subtitle discovery is unavailable for this video." }
        }
    }
    private var files: SubtitleFiles?
    private var generation = UUID()
    private var pending: Task<URL, any Error>?
    private var closed = false
    func choose(_ track: CoreSubtitle) async {
        guard !closed else { return }
        let token = UUID(); generation = token
        loading = true; error = nil
        do {
            if files == nil { files = try SubtitleFiles() }
            guard let files else { return }
            let previous = pending
            previous?.cancel()
            let task = Task {
                _ = await previous?.result
                try Task.checkCancellation()
                return try await files.load(track.url)
            }
            pending = task
            let next = try await withTaskCancellationHandler { try await task.value } onCancel: { task.cancel() }
            guard generation == token, !closed, !Task.isCancelled else { return }
            url = next; selectionID = token
        } catch {
            if generation == token, !closed { self.error = "This subtitle could not be loaded. Try another track." }
        }
        if generation == token { loading = false }
    }
    func disable() {
        generation = UUID(); pending?.cancel(); url = nil; selectionID = nil; loading = false; error = nil
    }
    func decoderFailed(selectionID rejected: UUID) {
        guard generation == rejected, selectionID == rejected else { return }
        disable(); error = "This subtitle could not be displayed. Try another track."
    }
    @discardableResult func stop() async -> Bool {
        closed = true; disable()
        discoveryGeneration = UUID(); discovery?.cancel()
        _ = await discovery?.result; discovery = nil; discovered = []; discoveryError = nil
        _ = await pending?.result; pending = nil
        do { try await files?.stop(); files = nil; return true }
        catch { self.error = "Subtitle cleanup could not finish. Retry Close."; return false }
    }
}
