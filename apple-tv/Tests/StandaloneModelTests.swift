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

@Test @MainActor func retiredActionCannotOverwriteRestartedGuestState() async throws {
    let service = NativeCoreService()
    let model = StandaloneModel(service: service)
    let first = Task { await model.run() }
    for _ in 0..<100 where model.snapshot?.status != .ready { try await Task.sleep(for: .milliseconds(10)) }
    let oldSession = model.snapshot?.session
    let gate = ActionGate()
    let action = Task { await model.perform { _, _ in
        await gate.wait()
        throw NativeCoreService.Failure.stopped
    } }
    for _ in 0..<100 where !(await gate.waiting) { try await Task.sleep(for: .milliseconds(10)) }
    #expect(await gate.waiting)
    first.cancel()
    let second = Task { await model.run() }
    await first.value
    for _ in 0..<100 where model.snapshot?.status != .ready || model.snapshot?.session == oldSession { try await Task.sleep(for: .milliseconds(10)) }
    #expect(model.snapshot?.session != oldSession)
    await gate.open()
    await action.value
    #expect(model.error == nil)
    second.cancel()
    await second.value
}

private actor ActionGate {
    private var continuation: CheckedContinuation<Void, Never>?
    var waiting: Bool { continuation != nil }
    func wait() async { await withCheckedContinuation { continuation = $0 } }
    func open() { continuation?.resume(); continuation = nil }
}

@Test @MainActor func commandAdmissionRejectsUnavailableAndInvalidSelections() async throws {
    let model = StandaloneModel(service: NativeCoreService())
    let unavailable = await model.perform { _, _ in Issue.record("Unavailable session must not dispatch") }
    #expect(!unavailable)
    let run = Task { await model.run() }
    for _ in 0..<100 where model.snapshot?.status != .ready { try await Task.sleep(for: .milliseconds(10)) }
    let rejected = await model.perform { service, revision in try await service.loadCatalog(index: -1, expectedRevision: revision) }
    #expect(!rejected)
    #expect(model.error != nil)
    let accepted = await model.perform { service, _ in try await service.setInterfaceLanguage("fra") }
    #expect(accepted)
    #expect(model.error == nil)
    run.cancel()
    await run.value
}

@Test @MainActor func stoppedCoreReportsStartupFailureAndAllowsRetry() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: directory) }
    let service = NativeCoreService(persistence: CorePersistenceConfiguration(directory: directory, key: Data(repeating: 61, count: 32)))
    let model = StandaloneModel(service: service)
    let first = Task { await model.run() }
    for _ in 0..<200 where model.error == nil { try await Task.sleep(for: .milliseconds(10)) }
    #expect(model.error != nil)
    first.cancel()
    await first.value
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    let retry = Task { await model.run() }
    for _ in 0..<200 where model.snapshot?.status != .ready { try await Task.sleep(for: .milliseconds(10)) }
    #expect(model.snapshot?.status == .ready)
    #expect(model.error == nil)
    retry.cancel()
    await retry.value
}

@Test @MainActor func replacingCoreJoinsOldOwnerAndRejectsItsLateAction() async throws {
    let old = NativeCoreService()
    let model = StandaloneModel(service: old)
    let first = Task { await model.run() }
    for _ in 0..<100 where model.snapshot?.status != .ready { try await Task.sleep(for: .milliseconds(10)) }
    let oldSession = try #require(model.snapshot?.session)
    let gate = ActionGate()
    let oldAction = Task { await model.perform { _, _ in await gate.wait(); throw NativeCoreService.Failure.stopped } }
    for _ in 0..<100 where !(await gate.waiting) { try await Task.sleep(for: .milliseconds(10)) }
    #expect(await gate.waiting)
    let replacement = NativeCoreService()
    try await model.replace(with: replacement)
    await first.value
    await #expect(throws: NativeCoreService.Failure.stopped) { _ = try await old.snapshot() }
    #expect(model.snapshot == nil)
    let second = Task { await model.run() }
    for _ in 0..<100 where model.snapshot?.status != .ready { try await Task.sleep(for: .milliseconds(10)) }
    #expect(model.snapshot?.session != oldSession)
    #expect(model.snapshot?.status == .ready)
    await gate.open()
    #expect(await oldAction.value == false)
    #expect(model.error == nil)
    second.cancel()
    await second.value
}

@Test @MainActor func signInFormFailureKeepsGuestAndCanCloseCleanly() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    let selection = CoreAccountSelection(keyAccount: "form-selection-" + UUID().uuidString)
    let form = AccountSignInModel(root: root, selection: selection)
    let guest = NativeCoreService()
    let home = StandaloneModel(service: guest)
    let run = Task { await home.run() }
    for _ in 0..<100 where home.snapshot?.status != .ready { try await Task.sleep(for: .milliseconds(10)) }
    let original = home.snapshot?.session
    form.submit(email: "", password: "synthetic-password", home: home) { Issue.record("Invalid credentials must not adopt an account") }
    for _ in 0..<300 where form.busy { try await Task.sleep(for: .milliseconds(10)) }
    #expect(!form.busy)
    #expect(form.error != nil)
    #expect(home.snapshot?.session == original)
    #expect(try await selection.load() == nil)
    await form.cancel()
    let children = (try? FileManager.default.contentsOfDirectory(atPath: root.path)) ?? []
    #expect(children.isEmpty)
    run.cancel()
    await run.value
}
