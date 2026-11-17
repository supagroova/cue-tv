import Foundation
import Testing
@testable import SittingRoomTV

@Suite struct PlaybackAccountingOwnerTests {
    @Test @MainActor func concurrentStartWaitsForOneNativeReceipt() async {
        let gate = AccountingStartGate()
        let owner = PlaybackAccountingOwner { action, _, _ in await gate.execute(action) }
        let first = Task { await owner.start() }
        while !(await gate.entered) { await Task.yield() }
        var secondEntered = false
        var secondFinished = false
        let second = Task {
            secondEntered = true
            let result = await owner.start()
            secondFinished = true
            return result
        }
        while !secondEntered { await Task.yield() }
        #expect(!secondFinished)
        await gate.release()
        #expect(await first.value)
        #expect(await second.value)
        #expect(await gate.starts == 1)
        #expect(await owner.close())
        #expect(!(await owner.start()))
    }

    @Test @MainActor func closeDuringStartWaitsThenPreventsDecoderActivation() async {
        let gate = AccountingStartGate()
        let owner = PlaybackAccountingOwner { action, _, _ in await gate.execute(action) }
        let starting = Task { await owner.start() }
        while !(await gate.entered) { await Task.yield() }
        var closeEntered = false
        var closeFinished = false
        let closing = Task {
            closeEntered = true
            let result = await owner.close()
            closeFinished = true
            return result
        }
        while !closeEntered { await Task.yield() }
        #expect(!closeFinished)
        await gate.release()
        #expect(!(await starting.value))
        #expect(await closing.value)
        let decoder = UUID()
        owner.activateDecoder(decoder)
        #expect(!owner.sample(time: 100, duration: 1000, state: .playing, decoder: decoder))
        #expect(await gate.starts == 1)
        #expect(await gate.closes == 0)
        #expect(await gate.retires == 1)
    }

    @Test @MainActor func closingBeforeValidTimingRetiresWithoutSaving() async {
        let recorder = AccountingRecorder()
        let owner = PlaybackAccountingOwner { action, time, duration in
            await recorder.record(action, time, duration)
        }
        #expect(await owner.start())
        let decoder = UUID()
        owner.activateDecoder(decoder)
        #expect(owner.sample(time: -1, duration: 0, state: .playing, decoder: decoder))
        #expect(owner.sample(time: 0, duration: 0, state: .other, decoder: decoder))
        #expect(await owner.close())
        #expect(await owner.close())
        #expect(await recorder.calls.map(\.action) == [.start, .retire])
    }

    @Test @MainActor func seekPauseAndPeriodicSaveKeepTheirOrder() async {
        let recorder = AccountingRecorder()
        var clock = ContinuousClock.now
        let owner = PlaybackAccountingOwner(now: { clock }) { action, time, duration in
            await recorder.record(action, time, duration)
        }
        #expect(await owner.start())
        let decoder = UUID()
        owner.activateDecoder(decoder)
        #expect(owner.sample(time: 100, duration: 2000, state: .playing, decoder: decoder))
        #expect(await owner.flush())
        let reposition = PlaybackReposition(id: UUID(), target: 1500)
        #expect(owner.sample(time: 200, duration: 2000, state: .playing, decoder: decoder, reposition: reposition))
        #expect(owner.sample(time: 1500, duration: 2000, state: .playing, decoder: decoder, reposition: reposition))
        #expect(owner.sample(time: 1600, duration: 2000, state: .playing, decoder: decoder, reposition: reposition))
        #expect(owner.sample(time: 1600, duration: 2000, state: .paused, decoder: decoder))
        #expect(await owner.flush())
        clock = clock.advanced(by: .seconds(6))
        #expect(owner.sample(time: 1650, duration: 2000, state: .playing, decoder: decoder))
        #expect(await owner.flush())
        clock = clock.advanced(by: .seconds(6))
        #expect(owner.sample(time: 1800, duration: 2000, state: .playing, decoder: decoder))
        #expect(!owner.sample(time: 1900, duration: 2000, state: .playing, decoder: UUID()))
        #expect(await owner.close())
        #expect(!owner.sample(time: 1900, duration: 2000, state: .playing, decoder: decoder))
        let calls = await recorder.calls
        #expect(calls.map(\.action) == [.start, .unpause, .seek, .checkpoint, .seek, .checkpoint, .played, .seek, .pause, .checkpoint, .unpause, .seek, .checkpoint, .played, .checkpoint, .close])
        #expect(calls.filter { $0.action == .seek }.map(\.time) == [100, 1500, 1600, 1650])
        #expect(calls.filter { $0.action == .played }.map(\.time) == [1600, 1800])
    }
}
private actor AccountingRecorder {
    struct Call: Sendable { let action: CorePlaybackAction; let time: UInt64; let duration: UInt64 }
    private(set) var calls: [Call] = []
    func record(_ action: CorePlaybackAction, _ time: UInt64, _ duration: UInt64) { calls.append(.init(action: action, time: time, duration: duration)) }
}

private actor AccountingStartGate {
    private(set) var entered = false
    private(set) var starts = 0
    private(set) var closes = 0
    private(set) var retires = 0
    private var continuation: CheckedContinuation<Void, Never>?
    func execute(_ action: CorePlaybackAction) async {
        if action == .close { closes += 1 }
        if action == .retire { retires += 1 }
        if action == .start {
            starts += 1; entered = true
            await withCheckedContinuation { continuation = $0 }
        }
    }
    func release() { continuation?.resume(); continuation = nil }
}
