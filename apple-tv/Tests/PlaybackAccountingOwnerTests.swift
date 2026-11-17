import Foundation
import Testing
@testable import SittingRoomTV

@Suite struct PlaybackAccountingOwnerTests {
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
