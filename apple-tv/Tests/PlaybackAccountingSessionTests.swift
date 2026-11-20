import Foundation
import Testing
@testable import SittingRoomTV

@Suite struct PlaybackAccountingSessionTests {
    @Test @MainActor func replacedUnstartedOwnerCannotStartFromLateViewTask() async throws {
        let session = PlaybackAccountingSession()
        let writer = SessionWriter()
        let first = try #require(await session.acquire(token: 41) { action, _, _ in try await writer.execute(action) })
        _ = try #require(await session.acquire(token: 42) { _, _, _ in nil })
        #expect(!(await first.start()))
        #expect(await writer.starts == 0)
        #expect(await session.close())
    }

    @Test @MainActor func failedSaveKeepsOwnerAndBlocksReplacementUntilRetry() async throws {
        let session = PlaybackAccountingSession()
        let writer = SessionWriter()
        let first = try #require(await session.acquire(token: 41) { action, _, _ in try await writer.execute(action) })
        #expect(await first.start())
        let decoder = UUID()
        first.activateDecoder(decoder)
        #expect(first.sample(time: 600, duration: 1000, state: .playing, decoder: decoder))
        #expect(await first.flush())
        #expect(!(await session.close()))
        let rejected = await session.acquire(token: 42) { _, _, _ in
            Issue.record("Replacement must not start while the prior save fails")
            return nil
        }
        #expect(rejected == nil)
        let retained = await session.acquire(token: 41) { _, _, _ in nil }
        #expect(retained === first)
        await writer.allowClose()
        let second = try #require(await session.acquire(token: 42) { action, _, _ in
            action == .pause ? CorePlaybackProgress(time: 0, duration: 0, timeWatched: 0, watched: false) : nil
        })
        #expect(second !== first)
        #expect(await writer.successfulCloses == 1)
        #expect(await second.start())
        #expect(await session.close())
    }
}
private actor SessionWriter {
    enum Failure: Error { case storage }
    private var canClose = false
    private(set) var successfulCloses = 0
    private(set) var starts = 0
    func allowClose() { canClose = true }
    func execute(_ action: CorePlaybackAction) throws -> CorePlaybackProgress? {
        if action == .start { starts += 1 }
        if action == .close {
            guard canClose else { throw Failure.storage }
            successfulCloses += 1
        }
        return action == .pause ? CorePlaybackProgress(time: 0, duration: 0, timeWatched: 0, watched: false) : nil
    }
}
