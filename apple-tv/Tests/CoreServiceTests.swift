import Foundation
import Testing
@testable import SittingRoomTV

@Suite struct CoreServiceTests {
@Test func swiftCoreServiceRunsRealModelAndJoinsStop() async throws {
    let service = NativeCoreService()
    try await service.start()
    var state = try await service.snapshot()
    for _ in 0..<100 where state.status != .ready {
        try await Task.sleep(for: .milliseconds(10))
        state = try await service.snapshot()
    }
    #expect(state.status == .ready)
    #expect(state.model?.interfaceLanguage == "eng")
    await #expect(throws: NativeCoreService.Failure.invalidRequest) { try await service.loadAddon("file:///private/manifest.json") }
    try await service.setInterfaceLanguage("fra")
    for _ in 0..<100 {
        state = try await service.snapshot()
        if state.model?.interfaceLanguage == "fra" && state.model?.pendingEffects == 0 { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.interfaceLanguage == "fra")
    #expect(state.model?.pendingEffects == 0)
    #expect(state.model?.error == nil)
    async let first: Void = service.stop()
    async let second: Void = service.stop()
    try await first
    try await second
    await #expect(throws: NativeCoreService.Failure.stopped) { try await service.setInterfaceLanguage("eng") }
    try await service.start()
    #expect(try await service.snapshot().session != state.session)
    try await service.stop()
}

}
