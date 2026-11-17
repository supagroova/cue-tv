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

@Test @MainActor func returningToGuestWaitsForPreviousShutdownThenRestarts() async throws {
    let service = NativeCoreService()
    let model = StandaloneModel(service: service)
    let first = Task { await model.run() }
    for _ in 0..<100 where model.snapshot?.status != .ready { try await Task.sleep(for: .milliseconds(10)) }
    let oldSession = model.snapshot?.session
    first.cancel()
    let second = Task { await model.run() }
    await first.value
    for _ in 0..<100 where model.snapshot?.status != .ready || model.snapshot?.session == oldSession {
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(model.snapshot?.status == .ready)
    #expect(model.snapshot?.session != oldSession)
    await model.perform { service, _ in try await service.setInterfaceLanguage("deu") }
    for _ in 0..<100 where model.snapshot?.model?.interfaceLanguage != "deu" { try await Task.sleep(for: .milliseconds(10)) }
    #expect(model.snapshot?.model?.interfaceLanguage == "deu")
    #expect(model.error == nil)
    second.cancel()
    await second.value
}
