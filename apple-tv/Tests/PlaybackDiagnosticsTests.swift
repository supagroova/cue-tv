import Foundation
import Testing
@testable import SittingRoomTV

@Test @MainActor func decoderBufferingRejectsInvalidAndRetiredCallbacks() async throws {
    let coordinator = VLCPlaybackSurface.Coordinator()
    coordinator.mediaPlayerBufferingChanged(0.4)
    for _ in 0..<20 where coordinator.bufferingProgress == nil { await Task.yield() }
    #expect(coordinator.bufferingProgress == 0.4)
    coordinator.mediaPlayerBufferingChanged(.nan)
    for _ in 0..<20 where coordinator.bufferingProgress != nil { await Task.yield() }
    #expect(coordinator.bufferingProgress == nil)
    coordinator.mediaPlayerBufferingChanged(0.9)
    coordinator.stop()
    try await Task.sleep(for: .milliseconds(20))
    #expect(coordinator.bufferingProgress == nil)
}
