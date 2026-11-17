import Foundation
import Observation

@MainActor @Observable
final class PlaybackSubtitlesModel {
    private(set) var url: URL?
    private(set) var selectionID: UUID?
    private(set) var error: String?
    private(set) var loading = false
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
        _ = await pending?.result; pending = nil
        do { try await files?.stop(); files = nil; return true }
        catch { self.error = "Subtitle cleanup could not finish. Retry Close."; return false }
    }
}
