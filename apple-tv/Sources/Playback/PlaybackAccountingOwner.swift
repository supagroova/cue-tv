import Foundation
import Observation

nonisolated struct PlaybackReposition: Sendable {
    let id: UUID
    let target: Int
}

/// Classifies decoder observations before they reach Core. The decoder's seek
/// marker remains pending until its observed clock reaches the requested region.
@MainActor @Observable
final class PlaybackAccountingOwner {
    enum State { case playing, paused, other }
    private let queue: PlaybackAccountingQueue
    private let now: @MainActor () -> ContinuousClock.Instant
    private var nextCheckpoint: ContinuousClock.Instant
    private var started = false
    private var ready = false
    private var closing = false
    private var decoder: UUID?
    private var lastTime: UInt64?
    private var lastPaused: Bool?
    private var repositionID: UUID?
    private var pendingTarget: Int?
    private var rejected: [PlaybackAccountingQueue.Operation]?
    var failed: Bool { queue.failed || rejected != nil }

    init(now: @escaping @MainActor () -> ContinuousClock.Instant = { .now },
         execute: @escaping @Sendable (CorePlaybackAction, UInt64, UInt64) async throws -> Void) {
        self.now = now
        nextCheckpoint = now().advanced(by: .seconds(5))
        queue = PlaybackAccountingQueue(execute: execute)
    }
    func start() async -> Bool {
        if started { return ready && !closing }
        started = true
        guard queue.enqueue(.start), await queue.flush() else { return false }
        ready = true
        return !closing
    }
    func flush() async -> Bool {
        guard await queue.flush() else { return false }
        return rejected == nil
    }
    /// Only the resource owner may activate a new decoder generation.
    func activateDecoder(_ token: UUID) {
        guard ready, !closing, decoder != token else { return }
        decoder = token; lastTime = nil; lastPaused = nil
        repositionID = nil; pendingTarget = nil
    }

    @discardableResult
    func sample(time: Int, duration: Int, state: State, decoder token: UUID,
                reposition: PlaybackReposition? = nil) -> Bool {
        guard ready, !closing, !failed, decoder == token else { return false }
        guard time >= 0 else { return true } // VLC reports a negative clock before timing exists.
        guard state != .other else { return true }
        let length = UInt64(max(0, duration))
        let position = length == 0 ? UInt64(time) : min(UInt64(time), length)
        if let reposition, reposition.id != repositionID {
            guard reposition.target >= 0 else { return false }
            repositionID = reposition.id; pendingTarget = reposition.target
        }
        var needsSeek = lastTime == nil || position < (lastTime ?? 0)
        if let target = pendingTarget {
            let bounded = length == 0 ? UInt64(target) : min(UInt64(target), length)
            let distance = position > bounded ? position - bounded : bounded - position
            guard distance <= 500 else { return true }
            needsSeek = true; pendingTarget = nil
        }
        var operations: [PlaybackAccountingQueue.Operation] = []
        let paused = state == .paused
        var saves = false
        if paused {
            if lastPaused != true || needsSeek {
                operations = [.init(.seek, time: position, duration: length), .init(.pause), .init(.checkpoint)]
                saves = true
            }
        } else {
            if lastPaused != false { operations.append(.init(.unpause)); needsSeek = true }
            operations.append(.init(needsSeek ? .seek : .played, time: position, duration: length))
            if needsSeek || now() >= nextCheckpoint { operations.append(.init(.checkpoint)); saves = true }
        }
        guard queue.enqueue(operations) else { rejected = operations; return false }
        lastTime = position; lastPaused = paused
        if saves { nextCheckpoint = now().advanced(by: .seconds(5)) }
        return true
    }

    func close() async -> Bool {
        closing = true
        guard started else { return true }
        if queue.failed, !(await queue.retry()) { return false }
        guard await queue.flush() else { return false }
        if let rejected {
            guard queue.enqueue(rejected), await queue.flush() else { return false }
            self.rejected = nil
        }
        // Repeated close observes the sealed queue; it never starts another owner.
        if !queue.enqueue(.close) { return await queue.flush() }
        return await queue.flush()
    }
}
