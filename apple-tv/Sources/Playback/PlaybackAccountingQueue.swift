import Foundation
import Observation

/// One playback owner's ordered native operations. Only adjacent, unsent played
/// samples coalesce; seek/pause/save/close remain barriers. Failures retain intent.
@MainActor @Observable
final class PlaybackAccountingQueue {
    struct Operation {
        let action: CorePlaybackAction
        let time: UInt64
        let duration: UInt64
        init(_ action: CorePlaybackAction, time: UInt64 = 0, duration: UInt64 = 0) {
            self.action = action; self.time = time; self.duration = duration
        }
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
        enqueue([Operation(action, time: time, duration: duration)])
    }

    /// A callback can require a seek/pause/save group. Admit the entire group or
    /// preserve the original queue, so capacity cannot split a lifecycle event.
    @discardableResult
    func enqueue(_ operations: [Operation]) -> Bool {
        guard !sealed, !failed else { return false }
        var next = pending
        var nextSealed = false
        for operation in operations {
            let action = operation.action
            guard !nextSealed, operation.time <= UInt64(Int64.max), operation.duration <= UInt64(Int64.max),
                  action == .seek || action == .played || (operation.time == 0 && operation.duration == 0) else { return false }
            if action == .played, next.last?.action == .played { next[next.count - 1] = operation }
            else { next.append(operation) }
            guard next.count + (inFlight ? 1 : 0) <= capacity else { return false }
            if action == .close || action == .retire { nextSealed = true }
        }
        pending = next; sealed = nextSealed
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
