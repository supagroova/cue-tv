import Foundation
import Observation

@MainActor @Observable
final class StandaloneModel {
    private(set) var snapshot: CoreSnapshot?
    private(set) var error: String?
    private var service: (any CoreService)?
    private var active: Task<Void, Never>?
    private var retirement: Task<Void, any Error>?
    private var generation = UUID()
    init(service: (any CoreService)? = nil) { self.service = service }

    func run() async {
        let token = UUID()
        generation = token
        let previous = active
        let pendingRetirement = retirement
        previous?.cancel()
        let task = Task { [weak self] in
            await previous?.value
            guard let self, !Task.isCancelled, self.generation == token else { return }
            do { try await pendingRetirement?.value }
            catch {
                if self.generation == token { self.error = "The previous session could not stop. Please retry." }
                return
            }
            guard !Task.isCancelled, self.generation == token else { return }
            await self.poll(generation: token)
        }
        active = task
        await withTaskCancellationHandler {
            await task.value
        } onCancel: { task.cancel() }
        if generation == token { active = nil }
    }

    /// Ownership transfers only on successful return; the caller retains a rejected candidate.
    func replace(with replacement: (any CoreService)?) async throws {
        let token = UUID()
        generation = token
        snapshot = nil
        let previous = active
        let oldService = service
        let pendingRetirement = retirement
        previous?.cancel()
        let retiring = Task {
            _ = try? await pendingRetirement?.value
            await previous?.value
            if let oldService { try await oldService.stop() }
        }
        retirement = retiring
        try await retiring.value
        guard generation == token, !Task.isCancelled else { throw CancellationError() }
        retirement = nil
        active = nil
        service = replacement
        snapshot = nil
        error = nil
    }

    private func poll(generation token: UUID) async {
        error = nil
        snapshot = nil
        do {
            if service == nil {
                let selected = try await NativeCoreService.selected()
                guard generation == token, !Task.isCancelled else { throw CancellationError() }
                service = selected
            }
            guard let service else { return }
            try await service.start()
            while !Task.isCancelled {
                let next = try await service.snapshot()
                guard generation == token, !Task.isCancelled else { throw CancellationError() }
                snapshot = next
                guard snapshot?.status != .stopped else { throw NativeCoreService.Failure.stopped }
                try await Task.sleep(for: .milliseconds(100))
            }
        } catch is CancellationError { }
        catch { self.error = "The Stremio session could not start. Please try again." }
        if let service { try? await service.stop() }
    }

    @discardableResult
    func perform(_ action: @Sendable (any CoreService, UInt64) async throws -> Void) async -> Bool {
        guard let service, let revision = snapshot?.model?.revision else { return false }
        let token = generation
        error = nil
        do {
            try await action(service, revision)
            return generation == token && !Task.isCancelled
        } catch {
            guard generation == token, !Task.isCancelled else { return false }
            self.error = "That selection is no longer available. Please try again."
            return false
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
