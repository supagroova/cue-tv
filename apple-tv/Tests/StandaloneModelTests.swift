import Foundation
import CryptoKit
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

@Test @MainActor func signingOutStopsAccountDeletesItsStorageAndPreservesOtherAccount() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    let selection = CoreAccountSelection(keyAccount: "signout-" + UUID().uuidString)
    let namespace = UUID()
    let storage = CoreStorage.account(id: namespace, root: root)
    let configuration = try await storage.prepare()
    let other = CoreStorage.account(id: UUID(), root: root)
    let otherConfiguration = try await other.prepare()
    defer { Task { try? await other.removeUnopenedAccount() } }
    try await selection.save(.init(namespace: namespace, userID: "synthetic-user"))
    let service = NativeCoreService(persistence: configuration, startup: .newAccount)
    let model = StandaloneModel(service: service)
    let run = Task { await model.run() }
    let startupDeadline = ContinuousClock.now.advanced(by: .seconds(3))
    while model.snapshot?.status != .ready, ContinuousClock.now < startupDeadline {
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(model.snapshot?.status == .ready)
    try await model.signOut(selection: selection, root: root)
    await run.value
    #expect(try await selection.load() == nil)
    #expect(!FileManager.default.fileExists(atPath: configuration.directory.path))
    #expect(FileManager.default.fileExists(atPath: otherConfiguration.directory.path))
    await #expect(throws: NativeCoreService.Failure.stopped) { _ = try await service.snapshot() }
    #expect(model.snapshot == nil)
    #expect(!model.signingOut)
}

@Test @MainActor func failedSignOutRetainsOwnerAndBlocksReplacementUntilCleanupRetry() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    let selection = CoreAccountSelection(keyAccount: "signout-retry-" + UUID().uuidString)
    let namespace = UUID()
    let storage = CoreStorage.account(id: namespace, root: root)
    let configuration = try await storage.prepare()
    try await selection.save(.init(namespace: namespace, userID: "synthetic-user"))
    let service = NativeCoreService(persistence: configuration, startup: .newAccount)
    let model = StandaloneModel(service: service)
    let run = Task { await model.run() }
    for _ in 0..<100 where model.snapshot?.status != .ready { try await Task.sleep(for: .milliseconds(10)) }
    #expect(model.snapshot?.status == .ready)
    // The storage boundary refuses removal through a mismatched directory.
    await #expect(throws: CoreStorage.Failure.unsafeDirectory) {
        try await model.signOut(selection: selection, root: root.appendingPathComponent("wrong-root"))
    }
    await run.value
    #expect(model.needsSignOutRetry)
    #expect(!model.signingOut)
    #expect(FileManager.default.fileExists(atPath: configuration.directory.path))
    #expect(try await selection.load() == nil)
    await #expect(throws: NativeCoreService.Failure.stopped) { try await model.replace(with: NativeCoreService()) }
    await model.run()
    #expect(model.snapshot == nil)
    #expect(model.error != nil)
    try await model.signOut(selection: selection, root: root)
    #expect(!model.needsSignOutRetry)
    #expect(!FileManager.default.fileExists(atPath: configuration.directory.path))
    try await model.replace(with: NativeCoreService())
}

@Test @MainActor func missingSavedAccountCanBeExplicitlyRemovedWithoutGuestFallback() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    let selection = CoreAccountSelection(keyAccount: "missing-account-recovery-" + UUID().uuidString)
    let namespace = UUID()
    try await selection.save(.init(namespace: namespace, userID: "synthetic-user"))
    let model = StandaloneModel(accountSelection: selection, accountRoot: root)
    await model.run()
    #expect(model.error != nil)
    #expect(model.snapshot == nil)
    #expect(model.canSignOut)
    #expect(try await selection.load()?.namespace == namespace)
    #expect(!FileManager.default.fileExists(atPath: root.path))
    try await model.signOut(selection: selection, root: root)
    #expect(try await selection.load() == nil)
    #expect(!model.canSignOut)
    #expect(model.error == nil)
    #expect(!FileManager.default.fileExists(atPath: root.path))
}

@Test @MainActor func encryptedAccountRestoresAcrossHomeInstancesThenSignsOut() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    let namespace = UUID()
    let storage = CoreStorage.account(id: namespace, root: root)
    let configuration = try await storage.prepare()
    let selection = CoreAccountSelection(keyAccount: "restored-home-" + UUID().uuidString)
    // Obtain the pinned Core settings schema through ordinary persisted guest state.
    let seed = NativeCoreService(persistence: configuration)
    try await seed.start()
    try await seed.setInterfaceLanguage("fra")
    var state = try await seed.snapshot()
    for _ in 0..<200 where state.model?.interfaceLanguage != "fra" || state.model?.pendingEffects != 0 {
        try await Task.sleep(for: .milliseconds(10))
        state = try await seed.snapshot()
    }
    #expect(state.model?.interfaceLanguage == "fra")
    try await seed.stop()
    let path = configuration.directory.appendingPathComponent("state.bin")
    let bytes = try Data(contentsOf: path)
    let header = Data("CUECORE1".utf8)
    #expect(bytes.prefix(8) == header)
    let key = SymmetricKey(data: configuration.key)
    let box = try AES.GCM.SealedBox(combined: Data(bytes.dropFirst(8)))
    let plaintext = try AES.GCM.open(box, using: key, authenticating: header)
    let values = try #require(JSONSerialization.jsonObject(with: plaintext) as? [String: Any])
    var profile = try #require(values["profile"] as? [String: Any])
    // Synthetic credentials never leave the test fixture. Restoration performs no login.
    profile["auth"] = ["key": "synthetic-restoration-token", "user": [
        "_id": "restoration-user", "email": "restore@example.test",
        "lastModified": "2026-01-01T00:00:00Z", "dateRegistered": "2026-01-01T00:00:00Z",
        "premium_expire": NSNull(),
        "gdpr_consent": ["tos": true, "privacy": true, "marketing": false, "from": NSNull()]
    ]]
    profile["addons"] = []
    let fixture = try JSONSerialization.data(withJSONObject: ["schema_version": 25, "profile": profile])
    let encrypted = try AES.GCM.seal(fixture, using: key, authenticating: header)
    try (header + #require(encrypted.combined)).write(to: path, options: .atomic)
    try await selection.save(.init(namespace: namespace, userID: "restoration-user"))
    var previousSession: UInt64?
    for cycle in 0..<2 {
        let model = StandaloneModel(accountSelection: selection, accountRoot: root)
        let run = Task { await model.run() }
        for _ in 0..<200 where model.snapshot?.status != .ready && model.error == nil {
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(model.error == nil)
        #expect(model.snapshot?.model?.authenticated == true)
        #expect(model.snapshot?.model?.interfaceLanguage == "fra")
        #expect(model.canSignOut)
        let session = try #require(model.snapshot?.session)
        #expect(session != previousSession)
        previousSession = session
        if cycle == 0 { try await model.replace(with: nil) }
        else { try await model.signOut() }
        await run.value
    }
    #expect(try await selection.load() == nil)
    #expect(!FileManager.default.fileExists(atPath: configuration.directory.path))
}

@Test @MainActor func staleDisplayedRevisionCannotDispatchAnEpisodeSelection() async throws {
    let service = NativeCoreService()
    let model = StandaloneModel(service: service)
    let run = Task { await model.run() }
    for _ in 0..<100 where model.snapshot?.status != .ready { try await Task.sleep(for: .milliseconds(10)) }
    let displayed = try #require(model.snapshot?.model?.revision)
    try await service.setInterfaceLanguage("fra")
    for _ in 0..<100 where model.snapshot?.model?.revision == displayed { try await Task.sleep(for: .milliseconds(10)) }
    #expect(model.snapshot?.model?.revision != displayed)
    let admitted = await model.perform(expectedRevision: displayed) { _, _ in
        Issue.record("A stale visible row must not dispatch against a replacement list")
    }
    #expect(!admitted)
    let source = await model.prepare(index: 0, expectedRevision: displayed)
    #expect(source == nil)
    run.cancel()
    await run.value
}

@Test @MainActor func refreshFormCannotReplaceGuestAndCanCancelWithoutRetiringIt() async throws {
    let service = NativeCoreService()
    let home = StandaloneModel(service: service)
    let run = Task { await home.run() }
    for _ in 0..<100 where home.snapshot?.status != .ready { try await Task.sleep(for: .milliseconds(10)) }
    let session = try #require(home.snapshot?.session)
    let refresh = AccountReauthenticationModel(home: home, session: session)
    refresh.submit(email: "fixture@example.test", password: "synthetic-password") {
        Issue.record("Guest cannot be adopted as a refreshed account")
    }
    for _ in 0..<100 where refresh.busy { try await Task.sleep(for: .milliseconds(10)) }
    #expect(refresh.error != nil)
    #expect(await refresh.cancel())
    #expect(home.snapshot?.session == session)
    #expect(home.snapshot?.model?.authenticated == false)
    #expect(try await service.snapshot().status == .ready)
    run.cancel()
    await run.value
}
