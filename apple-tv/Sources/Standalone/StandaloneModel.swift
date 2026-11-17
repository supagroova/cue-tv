import Foundation
import Observation

@MainActor @Observable
final class StandaloneModel {
    private(set) var snapshot: CoreSnapshot?
    private(set) var error: String?
    private var service: (any CoreService)?
    private var active: Task<Void, Never>?
    private var generation = UUID()
    init(service: (any CoreService)? = nil) { self.service = service }

    func run() async {
        let token = UUID()
        generation = token
        let previous = active
        previous?.cancel()
        let task = Task { [weak self] in
            await previous?.value
            guard let self, !Task.isCancelled, self.generation == token else { return }
            await self.poll()
        }
        active = task
        await withTaskCancellationHandler {
            await task.value
        } onCancel: { task.cancel() }
        if generation == token { active = nil }
    }

    private func poll() async {
        error = nil
        snapshot = nil
        do {
            if service == nil { service = try await NativeCoreService.guest() }
            guard let service else { return }
            try await service.start()
            while !Task.isCancelled {
                snapshot = try await service.snapshot()
                try await Task.sleep(for: .milliseconds(100))
            }
        } catch is CancellationError { }
        catch { self.error = "The Stremio session could not start. Please reopen the app." }
        if let service { try? await service.stop() }
    }

    func perform(_ action: @Sendable (any CoreService, UInt64) async throws -> Void) async {
        guard let service, let revision = snapshot?.model?.revision else { return }
        let token = generation
        error = nil
        do { try await action(service, revision) }
        catch {
            guard generation == token else { return }
            self.error = "That selection is no longer available. Please try again."
        }
    }

    func prepare(index: Int) async -> CoreTorrentRequest? {
        guard let service, let revision = snapshot?.model?.revision else { return nil }
        let token = generation
        do {
            let request = try await service.prepareTorrent(index: index, expectedRevision: revision)
            guard generation == token, !Task.isCancelled else { return nil }
            return request
        } catch {
            guard generation == token, !Task.isCancelled else { return nil }
            self.error = "This source could not be prepared. Select it again or choose another source."
            return nil
        }
    }
}
