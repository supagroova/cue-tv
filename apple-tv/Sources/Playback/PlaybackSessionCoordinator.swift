import Foundation
import Observation

/// Serializes resource retirement and rejects callbacks from retired generations.
@MainActor @Observable
final class PlaybackSessionCoordinator {
    enum Failure: Equatable {
        case invalidMetadata, discovery, buffering, read, decoder, unsupportedFormat, lowStorage, sourceTooLarge, cancellation

        static func native(code: String?) -> Self {
            switch code {
            case "invalid_metadata": .invalidMetadata
            case "unsupported_format": .unsupportedFormat
            case "low_storage": .lowStorage
            case "source_too_large": .sourceTooLarge
            default: .read
            }
        }

        var message: String {
            switch self {
            case .invalidMetadata: "This torrent could not be read. Choose another source."
            case .discovery: "No peers responded. Retry or choose another source."
            case .buffering: "Playback could not start. Retry or choose another source."
            case .read: "Playback data stopped arriving or could not be read. Retry or choose another source."
            case .decoder: "This video could not be decoded. Choose another source."
            case .unsupportedFormat: "This source format is not supported. Choose another source."
            case .lowStorage: "There is not enough free space for this video. Free space or choose a smaller source."
            case .sourceTooLarge: "This source exceeds the media cache limit. Choose a smaller source."
            case .cancellation: "Playback was canceled."
            }
        }
    }
    enum State: Equatable {
        case idle, resolving, discovering, choosingFile, buffering, playing, paused, seeking, stopping
        case failed(Failure)
    }
    private(set) var state: State = .idle
    private var generation = UUID()
    private var teardown: (@MainActor () async -> Void)?
    private var retirement: Task<Void, Never>?
    private var deadline: Task<Void, Never>?
    private var lastProgress: Int?
    private let wait: (Duration) async throws -> Void

    init(wait: @escaping (Duration) async throws -> Void = { try await Task.sleep(for: $0) }) {
        self.wait = wait
    }

    /// Allocate/start resources only after this returns a non-nil generation.
    func begin(teardown next: @escaping @MainActor () async -> Void) async -> UUID? {
        let token = retire()
        await retirement?.value
        guard generation == token else { return nil }
        teardown = next
        state = .resolving
        armDeadline()
        return token
    }

    func receive(_ next: State, generation token: UUID) {
        guard token == generation, teardown != nil else { return }
        if case .failed = state { return }
        // Only ownership methods may enter idle/stopping.
        guard next != .idle, next != .stopping, next != state else { return }
        if case .failed = next {
            _ = retire()
        }
        state = next
        lastProgress = nil
        armDeadline()
    }

    /// Only advancing decoder time refreshes the playing deadline; repeated
    /// polling, stale callbacks and byte downloads do not prove playback progress.
    func reportProgress(milliseconds: Int, generation token: UUID) {
        guard token == generation, teardown != nil, state == .playing,
              milliseconds >= 0, milliseconds > (lastProgress ?? -1) else { return }
        lastProgress = milliseconds
        armDeadline()
    }

    func stop() async {
        let token = retire()
        await retirement?.value
        guard generation == token else { return }
        state = .idle
    }

    private func armDeadline() {
        deadline?.cancel()
        let duration: Duration
        let failure: Failure
        switch state {
        case .resolving: duration = .seconds(15); failure = .invalidMetadata
        case .discovering: duration = .seconds(30); failure = .discovery
        case .buffering: duration = .seconds(30); failure = .buffering
        case .seeking: duration = .seconds(15); failure = .read
        case .playing: duration = .seconds(30); failure = .read
        default: deadline = nil; return
        }
        let token = generation
        let expected = state
        deadline = Task { [weak self, wait] in
            guard !Task.isCancelled else { return }
            do { try await wait(duration) } catch { return }
            guard !Task.isCancelled, let self, self.generation == token, self.state == expected else { return }
            self.receive(.failed(failure), generation: token)
        }
    }

    private func retire() -> UUID {
        deadline?.cancel(); deadline = nil
        lastProgress = nil
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
