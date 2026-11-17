import Foundation
import Testing
@testable import SittingRoomTV

@Test @MainActor func standaloneGuestUsesNativeCoreAndJoinsShutdown() async throws {
    let service = NativeCoreService()
    let model = StandaloneModel(service: service)
    let task = Task { await model.run() }
    for _ in 0..<100 where model.snapshot?.status != .ready {
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(model.snapshot?.status == .ready)
    #expect(model.error == nil)
    #expect((model.snapshot?.model?.catalogTotal ?? 0) > 0)
    await model.perform { service, _ in try await service.setInterfaceLanguage("fra") }
    for _ in 0..<100 where model.snapshot?.model?.interfaceLanguage != "fra" {
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(model.snapshot?.model?.interfaceLanguage == "fra")
    task.cancel()
    await task.value
    await #expect(throws: NativeCoreService.Failure.self) { try await service.snapshot() }
}
