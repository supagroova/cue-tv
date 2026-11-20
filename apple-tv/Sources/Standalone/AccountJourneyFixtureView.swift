#if DEBUG && targetEnvironment(simulator)
import SwiftUI
import Security
import CoreFoundation

/// Creates a real paused synthetic account; all subsequent controls are production UI.
struct AccountJourneyFixtureView: View {
    private struct Configuration { let root: URL; let selection: CoreAccountSelection }
    @State private var configuration: Configuration?
    @State private var leftovers = false
    @State private var model: StandaloneModel?
    @State private var failed = false
    @State private var cleared = false
    @State private var storageFault: AccountJourneyStorageFault?
    var body: some View {
        Group {
            if cleared { Text(leftovers ? "Account journey cleared leftover data" : "Account journey storage cleared") }
            else if let model, let configuration {
                StandaloneHome(model: model, makeSignIn: {
                    AccountSignInModel(root: configuration.root, selection: configuration.selection, transport: .fixture(42429))
                })
            }
            else if failed { Text("Account journey fixture could not start") }
            else { ProgressView("Preparing synthetic account…") }
        }
        .task {
            guard model == nil, !cleared else { return }
            var owner: NativeCoreService?
            do {
                let environment = ProcessInfo.processInfo.environment
                guard let raw = environment["CUE_ACCOUNT_JOURNEY_STORAGE"], let namespace = UUID(uuidString: raw) else {
                    throw CoreStorage.Failure.unsafeDirectory
                }
                let root = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
                    .appendingPathComponent("CueAccountJourneyTests", isDirectory: true)
                    .appendingPathComponent(namespace.uuidString.lowercased(), isDirectory: true)
                let storage = CoreStorage.account(id: namespace, root: root)
                let selection = CoreAccountSelection(keyAccount: "account-journey-" + namespace.uuidString.lowercased())
                if environment["CUE_ACCOUNT_JOURNEY_CLEAR"] == "1" {
                    let original = root.appendingPathComponent(namespace.uuidString.lowercased(), isDirectory: true)
                    let held = original.appendingPathExtension("held")
                    if FileManager.default.fileExists(atPath: held.path) {
                        try FileManager.default.moveItem(at: held, to: original)
                    }
                    let selected: CoreAccountSelection.Account?
                    do { selected = try await selection.load() }
                    catch CoreAccountSelection.Failure.invalidRecord {
                        try await selection.clearInvalidRecord()
                        selected = nil
                    }
                    let entries = FileManager.default.fileExists(atPath: root.path)
                        ? try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil) : []
                    guard entries.allSatisfy({ UUID(uuidString: $0.lastPathComponent) != nil }) else {
                        throw CoreStorage.Failure.unsafeDirectory
                    }
                    leftovers = environment["CUE_ACCOUNT_JOURNEY_EXPECT_EMPTY"] == "1" && (selected != nil || !entries.isEmpty)
                    if let selected { try await selection.clear(ifMatching: selected.namespace) }
                    for entry in entries {
                        guard let id = UUID(uuidString: entry.lastPathComponent) else { throw CoreStorage.Failure.unsafeDirectory }
                        try await CoreStorage.account(id: id, root: root).removeUnopenedAccount()
                    }
                    if FileManager.default.fileExists(atPath: root.path) { try FileManager.default.removeItem(at: root) }
                    cleared = true
                    return
                }
                configuration = Configuration(root: root, selection: selection)
                if environment["CUE_ACCOUNT_JOURNEY_INVALID_SELECTION"] == "1" {
                    let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
                        kSecAttrService as String: CoreStorage.keyService,
                        kSecAttrAccount as String: "account-journey-" + namespace.uuidString.lowercased(),
                        kSecValueData as String: Data("invalid-json".utf8)]
                    guard SecItemAdd(query as CFDictionary, nil) == errSecSuccess else {
                        throw CoreAccountSelection.Failure.invalidRecord
                    }
                    model = StandaloneModel(accountSelection: selection, accountRoot: root)
                    return
                }
                if environment["CUE_ACCOUNT_JOURNEY_FIRST_SIGN_IN"] == "1" {
                    guard try await selection.load() == nil else { throw CoreAccountSelection.Failure.alreadySelected }
                    model = StandaloneModel(service: NativeCoreService(), accountSelection: selection, accountRoot: root)
                    return
                }
                let persistence = try await storage.prepare()
                let service = NativeCoreService(persistence: persistence, startup: .fixtureAccount(42429))
                owner = service
                try await service.start()
                _ = try await waitFor(service) { $0.status == .ready }
                try await service.login(email: "fixture@example.test", password: "fixture-password")
                var state = try await waitFor(service) { $0.model?.authenticated == true && $0.model?.pendingEffects == 0 }
                guard try await service.accountIdentity() == "fixture-user",
                      let choice = state.model?.catalogs.first, let revision = state.model?.revision else {
                    throw NativeCoreService.Failure.malformedSnapshot
                }
                try await selection.save(.init(namespace: namespace, userID: "fixture-user"))
                try await service.loadCatalog(index: choice.index, expectedRevision: revision)
                state = try await waitFor(service) { $0.model?.catalog?.status == .ready }
                guard let revision = state.model?.revision else { throw NativeCoreService.Failure.malformedSnapshot }
                try await service.loadDetails(index: 0, expectedRevision: revision)
                state = try await waitFor(service) { $0.model?.details?.status == .ready }
                guard let revision = state.model?.revision else { throw NativeCoreService.Failure.malformedSnapshot }
                try await service.setDetailsLibrary(true, expectedRevision: revision)
                _ = try await waitFor(service) { $0.model?.sync?.state == .reauthenticationRequired && $0.model?.sync?.pendingTitles == 1 }
                if environment["CUE_ACCOUNT_JOURNEY_FAILED_SAVE"] == "1" {
                    let held = persistence.directory.appendingPathExtension("held")
                    try FileManager.default.moveItem(at: persistence.directory, to: held)
                    storageFault = AccountJourneyStorageFault(original: persistence.directory, held: held, namespace: namespace)
                }
                model = StandaloneModel(service: service, accountSelection: selection, accountRoot: root)
            } catch {
                try? await owner?.stop()
                failed = true
            }
        }
    }
    private func waitFor(_ service: NativeCoreService, matching predicate: (CoreSnapshot) -> Bool) async throws -> CoreSnapshot {
        for _ in 0..<200 {
            try Task.checkCancellation()
            let state = try await service.snapshot()
            if predicate(state) { return state }
            try await Task.sleep(for: .milliseconds(25))
        }
        throw NativeCoreService.Failure.requestTimeout
    }
}
/// Keeps fault repair outside production focus/navigation and scopes it to one fixture.
nonisolated private final class AccountJourneyStorageFault {
    let original: URL
    let held: URL
    let notification: CFNotificationName
    init(original: URL, held: URL, namespace: UUID) {
        self.original = original
        self.held = held
        notification = CFNotificationName("cue.account.restore.\(namespace.uuidString)" as CFString)
        CFNotificationCenterAddObserver(CFNotificationCenterGetDarwinNotifyCenter(), Unmanaged.passUnretained(self).toOpaque(),
            { _, observer, _, _, _ in
                guard let observer else { return }
                let fault = Unmanaged<AccountJourneyStorageFault>.fromOpaque(observer).takeUnretainedValue()
                // A failed repair leaves the real storage fault in place; UI retry must still fail.
                if FileManager.default.fileExists(atPath: fault.held.path) {
                    try? FileManager.default.moveItem(at: fault.held, to: fault.original)
                }
            }, notification.rawValue, nil, .deliverImmediately)
    }
    deinit {
        CFNotificationCenterRemoveObserver(CFNotificationCenterGetDarwinNotifyCenter(), Unmanaged.passUnretained(self).toOpaque(), notification, nil)
    }
}

#endif
