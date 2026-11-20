import Foundation
import Testing
@testable import SittingRoomTV

@Suite struct PlaybackAccountingQueueTests {
    @Test @MainActor func coalescesProgressBehindHeldWriteWithoutDroppingSeekOrClose() async {
        let gate = AccountingGate()
        let queue = PlaybackAccountingQueue(capacity: 6) { action, time, duration in
            try await gate.execute(action, time, duration)
        }
        #expect(queue.enqueue(.start))
        while !(await gate.entered) { await Task.yield() }
        for time in 1...1000 { #expect(queue.enqueue(.played, time: UInt64(time), duration: 2000)) }
        #expect(queue.enqueue(.seek, time: 1500, duration: 2000))
        #expect(queue.enqueue(.played, time: 1600, duration: 2000))
        #expect(queue.enqueue(.checkpoint))
        #expect(queue.enqueue(.close))
        #expect(!queue.enqueue(.played, time: 1700, duration: 2000))
        #expect(queue.pendingCount == 6)
        await gate.release()
        #expect(await queue.flush())
        let calls = await gate.calls
        #expect(calls.map(\.action) == [.start, .played, .seek, .played, .checkpoint, .close])
        #expect(calls.map(\.time) == [0, 1000, 1500, 1600, 0, 0])
        #expect(queue.pendingCount == 0)
    }

    @Test @MainActor func capacityRejectionPreservesQueuedBarrierAndDoesNotSealClose() async {
        let gate = AccountingGate()
        let queue = PlaybackAccountingQueue(capacity: 2) { action, time, duration in
            try await gate.execute(action, time, duration)
        }
        #expect(queue.enqueue(.start))
        while !(await gate.entered) { await Task.yield() }
        #expect(queue.enqueue(.seek, time: 500, duration: 1000))
        #expect(!queue.enqueue(.close))
        #expect(!queue.enqueue([.init(.pause), .init(.checkpoint)]))
        #expect(queue.pendingCount == 2)
        await gate.release()
        #expect(await queue.flush())
        #expect(queue.enqueue(.close))
        #expect(await queue.flush())
        let calls = await gate.calls
        #expect(calls.map(\.action) == [.start, .seek, .close])
        #expect(calls.map(\.time) == [0, 500, 0])
    }

    @Test @MainActor func callbackBatchCannotPartiallyFit() async {
        let gate = AccountingGate()
        let queue = PlaybackAccountingQueue(capacity: 2) { action, time, duration in
            try await gate.execute(action, time, duration)
        }
        #expect(queue.enqueue(.start))
        while !(await gate.entered) { await Task.yield() }
        #expect(!queue.enqueue([.init(.seek, time: 500, duration: 1000), .init(.checkpoint)]))
        #expect(queue.pendingCount == 1)
        #expect(queue.enqueue(.close))
        await gate.release()
        #expect(await queue.flush())
        let calls = await gate.calls
        #expect(calls.map(\.action) == [.start, .close])
    }

    @Test @MainActor func failedCloseRetainsIntentAndRetryMustFinishBeforeSuccess() async {
        let gate = AccountingGate(held: false, failClose: true)
        let queue = PlaybackAccountingQueue { action, time, duration in
            try await gate.execute(action, time, duration)
        }
        #expect(queue.enqueue(.played, time: 600, duration: 1000))
        #expect(queue.enqueue(.close))
        #expect(!(await queue.flush()))
        #expect(queue.failed)
        #expect(queue.pendingCount == 1)
        #expect(!queue.enqueue(.played, time: 900, duration: 1000))
        await gate.allowClose()
        #expect(await queue.retry())
        #expect(!queue.failed)
        #expect(queue.pendingCount == 0)
        let calls = await gate.calls
        #expect(calls.map(\.action) == [.played, .close, .close])
    }
}

private actor AccountingGate {
    struct Call: Sendable { let action: CorePlaybackAction; let time: UInt64; let duration: UInt64 }
    enum Failure: Error { case write }
    private var held: Bool
    private var failClose: Bool
    private var continuation: CheckedContinuation<Void, Never>?
    private(set) var entered = false
    private(set) var calls: [Call] = []
    init(held: Bool = true, failClose: Bool = false) { self.held = held; self.failClose = failClose }
    func execute(_ action: CorePlaybackAction, _ time: UInt64, _ duration: UInt64) async throws {
        calls.append(.init(action: action, time: time, duration: duration))
        entered = true
        if held { await withCheckedContinuation { continuation = $0 } }
        if action == .close && failClose { throw Failure.write }
    }
    func release() { held = false; continuation?.resume(); continuation = nil }
    func allowClose() { failClose = false }
}
