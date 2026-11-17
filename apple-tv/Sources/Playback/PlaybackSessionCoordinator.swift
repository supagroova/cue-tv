import Foundation
import Observation

/// Serializes resource retirement and rejects callbacks from retired generations.
@MainActor @Observable
final class PlaybackSessionCoordinator {
    enum Failure: Equatable {
        case invalidMetadata, discovery, buffering, read, decoder, unsupportedFormat, lowStorage, cancellation
    }
    enum State: Equatable {
        case idle, resolving, discovering, buffering, playing, paused, seeking, stopping
        case failed(Failure)
    }
    private(set) var state: State = .idle
    private var generation = UUID()
    private var teardown: (@MainActor () async -> Void)?
    private var retirement: Task<Void, Never>?

    /// Allocate/start resources only after this returns a non-nil generation.
    func begin(teardown next: @escaping @MainActor () async -> Void) async -> UUID? {
        let token = retire()
        await retirement?.value
        guard generation == token else { return nil }
        teardown = next
        state = .resolving
        return token
    }

    func receive(_ next: State, generation token: UUID) {
        guard token == generation, teardown != nil else { return }
        if case .failed = state { return }
        // Only ownership methods may enter idle/stopping.
        guard next != .idle, next != .stopping else { return }
        state = next
    }

    func stop() async {
        let token = retire()
        await retirement?.value
        guard generation == token else { return }
        state = .idle
    }

    private func retire() -> UUID {
        generation = UUID()
        let previous = retirement
        let release = teardown
        teardown = nil
        state = .stopping
        retirement = Task {
            await previous?.value
            await release?()
        }
        return generation
    }
}
