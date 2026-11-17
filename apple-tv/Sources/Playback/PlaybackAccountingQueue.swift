import Foundation
import Observation

/// One playback owner's ordered native operations. Only adjacent, unsent played
/// samples coalesce; seek/pause/save/close remain barriers. Failures retain intent.
@MainActor @Observable
final class PlaybackAccountingQueue {
    private struct Operation {
        let action: CorePlaybackAction
        let time: UInt64
        let duration: UInt64
    }
    private let capacity: Int
    private let execute: @Sendable (CorePlaybackAction, UInt64, UInt64) async throws -> Void
    private var pending: [Operation] = []
    private var worker: Task<Void, Never>?
    private var inFlight = false
    private var sealed = false
    private var waiters: [CheckedContinuation<Bool, Never>] = []
    private(set) var failed = false
    var pendingCount: Int { pending.count + (inFlight ? 1 : 0) }

    init(capacity: Int = 32, execute: @escaping @Sendable (CorePlaybackAction, UInt64, UInt64) async throws -> Void) {
        self.capacity = max(1, capacity)
        self.execute = execute
    }

    /// False means the caller must stop admission and surface backpressure; no
    /// existing operation is evicted to make room for a lifecycle barrier.
    @discardableResult
    func enqueue(_ action: CorePlaybackAction, time: UInt64 = 0, duration: UInt64 = 0) -> Bool {
        guard !sealed, !failed, time <= UInt64(Int64.max), duration <= UInt64(Int64.max),
              action == .seek || action == .played || (time == 0 && duration == 0) else { return false }
        let operation = Operation(action: action, time: time, duration: duration)
        if action == .played, pending.last?.action == .played {
            pending[pending.count - 1] = operation
        } else {
            guard pendingCount < capacity else { return false }
            pending.append(operation)
        }
        if action == .close || action == .retire { sealed = true }
        startWorker()
        return true
    }

    func flush() async -> Bool {
        if failed { return false }
        if pendingCount == 0 { return true }
        guard waiters.count < capacity else { return false }
        return await withCheckedContinuation { waiters.append($0) }
    }

    func retry() async -> Bool {
        if failed { failed = false; startWorker() }
        return await flush()
    }

    private func startWorker() {
        guard worker == nil, !failed, !pending.isEmpty else { return }
        worker = Task { await drain() }
    }

    private func drain() async {
        while !pending.isEmpty {
            let operation = pending.removeFirst()
            inFlight = true
            do {
                try await execute(operation.action, operation.time, operation.duration)
                inFlight = false
            } catch {
                inFlight = false
                pending.insert(operation, at: 0)
                failed = true
                worker = nil
                finishWaiters(false)
                return
            }
        }
        worker = nil
        finishWaiters(true)
    }

    private func finishWaiters(_ result: Bool) {
        let completions = waiters
        waiters.removeAll()
        for completion in completions { completion.resume(returning: result) }
    }
}
