import Foundation
import Network
import Security
import Testing
@testable import SittingRoomTV

@Test func syntheticAccountTraversesSwiftFFIHTTPAndEncryptedReopen() async throws {
    let fixture = try await AccountHTTPFixture.start()
    defer { fixture.listener.cancel() }
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let storage = CoreStorage.account(id: UUID(), root: root)
    let configuration = try await storage.prepare()
    let service = NativeCoreService(persistence: configuration, startup: .fixtureAccount(fixture.port))
    try await service.start()
    var state = try await service.snapshot()
    for _ in 0..<200 where state.status != .ready {
        try await Task.sleep(for: .milliseconds(10))
        state = try await service.snapshot()
    }
    #expect(state.status == .ready)
    try await service.login(email: "fixture@example.test", password: "fixture-password")
    for _ in 0..<300 {
        state = try await service.snapshot()
        if state.model?.authenticated == true && state.model?.pendingEffects == 0 { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.authenticated == true)
    #expect(state.model?.pendingEffects == 0)
    #expect(try await service.accountIdentity() == "fixture-user")
    #expect(await fixture.probe.password == "fixture-password")
    try await service.stop()
    let encrypted = try Data(contentsOf: configuration.directory.appendingPathComponent("state.bin"))
    #expect(encrypted.starts(with: Data("CUECORE1".utf8)))
    #expect(encrypted.range(of: Data("fixture-token".utf8)) == nil)
    let restored = NativeCoreService(persistence: configuration, startup: .restoreAccount("fixture-user"))
    try await restored.start()
    var reopened = try await restored.snapshot()
    for _ in 0..<200 where reopened.status != .ready {
        try await Task.sleep(for: .milliseconds(10))
        reopened = try await restored.snapshot()
    }
    #expect(reopened.model?.authenticated == true)
    #expect(try await restored.accountIdentity() == "fixture-user")
    try await restored.discardAccountStorage(storage)
    try? FileManager.default.removeItem(at: root)
}

@Test(arguments: ["none", "identity", "storage", "cancel"]) @MainActor
func accountRefreshFormResumesRejectedSyncWithoutChangingOwner(failure: String) async throws {
    let fixture = try await AccountHTTPFixture.start(recovery: true, rejectFirstRefresh: failure == "identity", holdFirstRefresh: failure == "cancel")
    defer { fixture.listener.cancel() }
    let account = try await pausedAccount(fixture)
    let (root, storage, configuration, service) = (account.root, account.storage, account.configuration, account.service)
    var state = account.state
    let originalSession = state.session
    let home = StandaloneModel(service: service)
    let run = Task { await home.run() }
    for _ in 0..<200 where home.snapshot?.model?.sync?.state != .reauthenticationRequired {
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(home.snapshot?.session == originalSession)
    let form = AccountReauthenticationModel(home: home, session: originalSession)
    var completions = 0
    let heldDirectory = configuration.directory.appendingPathExtension("held")
    if failure == "storage" { try FileManager.default.moveItem(at: configuration.directory, to: heldDirectory) }
    defer {
        if FileManager.default.fileExists(atPath: heldDirectory.path) {
            try? FileManager.default.moveItem(at: heldDirectory, to: configuration.directory)
        }
    }
    if failure != "none" {
        form.submit(email: "fixture@example.test", password: "replacement-password") { completions += 1 }
        if failure == "cancel" {
            for _ in 0..<200 where await fixture.probe.loginCount < 2 { try await Task.sleep(for: .milliseconds(10)) }
            #expect(await fixture.probe.loginCount == 2)
            let started = ContinuousClock.now
            #expect(await form.cancel())
            #expect(started.duration(to: .now) < .seconds(1))
            for _ in 0..<100 where await fixture.probe.closedRefreshes == 0 { try await Task.sleep(for: .milliseconds(10)) }
            #expect(await fixture.probe.closedRefreshes == 1)
        } else {
            for _ in 0..<400 where form.busy { try await Task.sleep(for: .milliseconds(10)) }
        }
        #expect(!form.busy)
        #expect(completions == 0)
        #expect(form.canSubmit)
        if failure == "cancel" { #expect(form.error == nil) }
        else {
            #expect(form.error == (failure == "identity"
                ? "Use the same Stremio account. Your pending changes have been kept."
                : "Sign-in could not be saved. Your pending changes have been kept."))
        }
        let failed = try await service.snapshot()
        #expect(failed.session == originalSession)
        #expect(failed.model?.sync?.state == .reauthenticationRequired)
        #expect(failed.model?.sync?.pendingTitles == 1)
        #expect(try await service.accountIdentity() == "fixture-user")
        #expect(await fixture.probe.uploads.count == 1)
        if failure == "storage" { try FileManager.default.moveItem(at: heldDirectory, to: configuration.directory) }
    }
    form.submit(email: "fixture@example.test", password: "replacement-password") { completions += 1 }
    for _ in 0..<400 where form.busy { try await Task.sleep(for: .milliseconds(10)) }
    #expect(!form.busy)
    #expect(form.error == nil)
    #expect(completions == 1)
    #expect(!form.canSubmit)
    state = try await accountState(service) { $0.model?.sync?.pendingTitles == 0 }
    #expect(state.session == originalSession)
    #expect(state.model?.detailsInLibrary == true)
    #expect(try await service.accountIdentity() == "fixture-user")
    let uploads = await fixture.probe.uploads
    #expect(uploads.map(\.authKey) == ["fixture-token", "replacement-token"])
    #expect(uploads.map { $0.changes.map(\.id) } == [["account-title"], ["account-title"]])
    #expect(await fixture.probe.loginCount == (failure == "none" ? 2 : 3))
    #expect(await form.cancel())
    run.cancel()
    await run.value
    try await service.discardAccountStorage(storage)
    try? FileManager.default.removeItem(at: root)
}

@Test(arguments: [false, true]) @MainActor
func signOutRetiresPendingSyncAndHeldRefreshPreservingOtherAccount(failCleanup: Bool) async throws {
    let fixture = try await AccountHTTPFixture.start(recovery: true, holdFirstRefresh: true)
    defer { fixture.listener.cancel() }
    let account = try await pausedAccount(fixture)
    let selection = CoreAccountSelection(keyAccount: "pending-signout-" + UUID().uuidString)
    let keyQuery: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: CoreStorage.keyService,
        kSecAttrAccount as String: "account-v1:" + account.namespace.uuidString.lowercased()
    ]
    try await selection.save(.init(namespace: account.namespace, userID: "fixture-user"))
    let other = CoreStorage.account(id: UUID(), root: account.root)
    let otherConfiguration = try await other.prepare()
    let sentinel = Data("other-account-retained".utf8)
    let sentinelURL = otherConfiguration.directory.appendingPathComponent("sentinel")
    try sentinel.write(to: sentinelURL)
    let home = StandaloneModel(service: account.service, accountSelection: selection, accountRoot: account.root)
    let run = Task { await home.run() }
    for _ in 0..<200 where home.snapshot?.model?.sync?.state != .reauthenticationRequired {
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(home.snapshot?.session == account.state.session)
    let form = AccountReauthenticationModel(home: home, session: account.state.session)
    var completions = 0
    form.submit(email: "fixture@example.test", password: "replacement-password") { completions += 1 }
    for _ in 0..<200 where await fixture.probe.loginCount < 2 { try await Task.sleep(for: .milliseconds(10)) }
    #expect(await fixture.probe.loginCount == 2)
    let stateURL = account.configuration.directory.appendingPathComponent("state.bin")
    let retainedState = try Data(contentsOf: stateURL)
    let heldDirectory = account.configuration.directory.appendingPathExtension("held")
    if failCleanup {
        // Corrupt only this fixture's path. Cleanup must reject the symlink,
        // retain its account owner, and never delete the other account target.
        try FileManager.default.moveItem(at: account.configuration.directory, to: heldDirectory)
        try FileManager.default.createSymbolicLink(at: account.configuration.directory, withDestinationURL: otherConfiguration.directory)
        await #expect(throws: CoreStorage.Failure.unsafeDirectory) { try await home.signOut() }
        #expect(home.needsSignOutRetry)
        #expect(SecItemCopyMatching(keyQuery as CFDictionary, nil) == errSecSuccess)
        #expect(try Data(contentsOf: heldDirectory.appendingPathComponent("state.bin")) == retainedState)
        #expect(try Data(contentsOf: sentinelURL) == sentinel)
        await #expect(throws: NativeCoreService.Failure.stopped) { try await home.replace(with: NativeCoreService()) }
        try FileManager.default.removeItem(at: account.configuration.directory)
        try FileManager.default.moveItem(at: heldDirectory, to: account.configuration.directory)
    }
    try await home.signOut()
    await run.value
    #expect(await form.cancel())
    for _ in 0..<100 where await fixture.probe.closedRefreshes == 0 { try await Task.sleep(for: .milliseconds(10)) }
    #expect(await fixture.probe.closedRefreshes == 1)
    #expect(completions == 0)
    #expect(await fixture.probe.uploads.count == 1)
    #expect(try await selection.load() == nil)
    #expect(!home.needsSignOutRetry)
    #expect(home.snapshot == nil)
    #expect(!FileManager.default.fileExists(atPath: account.configuration.directory.path))
    #expect(SecItemCopyMatching(keyQuery as CFDictionary, nil) == errSecItemNotFound)
    await #expect(throws: CoreStorage.Failure.missingState) { _ = try await account.storage.prepareExistingAccount() }
    await #expect(throws: NativeCoreService.Failure.stopped) { _ = try await account.service.snapshot() }
    #expect(try await other.prepare().key == otherConfiguration.key)
    #expect(try Data(contentsOf: sentinelURL) == sentinel)
    try await other.removeUnopenedAccount()
    try FileManager.default.removeItem(at: account.root)
}

@Test @MainActor func mediaCachePurgeRestartsPeerTransferWithoutTouchingPendingAccount() async throws {
    let fixture = try await AccountHTTPFixture.start(recovery: true)
    defer { fixture.listener.cancel() }
    let account = try await pausedAccount(fixture)
    let encryptedURL = account.configuration.directory.appendingPathComponent("state.bin")
    let encryptedBefore = try Data(contentsOf: encryptedURL)
    let (tracker, port) = try await startFixtureTracker()
    defer { tracker.cancel() }
    let mediaRoot = account.root.appendingPathComponent("media-cache")
    let cache = TorrentCacheStore(root: mediaRoot)
    let model = SourcePlaybackModel(cache: cache)
    let request = CoreTorrentRequest(metaId: "account-title", videoId: "account-title", kind: "movie",
        magnet: "magnet:?xt=urn:btih:1be994f3f99cc80c9c2830b113432065d1551e9d&tr=http://127.0.0.1:\(port)/announce",
        fileIndex: 0, fileMustInclude: [])
    let client = URLSession(configuration: .ephemeral)
    defer { client.invalidateAndCancel() }
    var originalBytes: Data?
    for cycle in 0..<2 {
        await model.start(.torrent(request))
        let url = try #require(model.playbackURL)
        let (bytes, _) = try await client.data(for: URLRequest(url: url, cachePolicy: .reloadIgnoringLocalCacheData))
        #expect(bytes.count == 175917)
        if let originalBytes { #expect(bytes == originalBytes) } else { originalBytes = bytes }
        await model.stop()
        #expect(!model.cleanupFailed)
        let state = try await account.service.snapshot()
        #expect(state.session == account.state.session)
        #expect(state.model?.authenticated == true)
        #expect(state.model?.detailsInLibrary == true)
        #expect(state.model?.sync?.state == .reauthenticationRequired)
        #expect(state.model?.sync?.pendingTitles == 1)
        #expect(try Data(contentsOf: encryptedURL) == encryptedBefore)
        #expect(try await account.storage.prepareExistingAccount().key == account.configuration.key)
        if cycle == 0 { try FileManager.default.removeItem(at: mediaRoot) }
    }
    #expect(await fixture.probe.uploads.count == 1)
    try await account.service.discardAccountStorage(account.storage)
    try FileManager.default.removeItem(at: account.root)
}

private func pausedAccount(_ fixture: AccountHTTPFixture) async throws -> (
    root: URL, namespace: UUID, storage: CoreStorage, configuration: CorePersistenceConfiguration,
    service: NativeCoreService, state: CoreSnapshot
) {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let namespace = UUID()
    let storage = CoreStorage.account(id: namespace, root: root)
    let configuration = try await storage.prepare()
    let service = NativeCoreService(persistence: configuration, startup: .fixtureAccount(fixture.port))
    try await service.start()
    _ = try await accountState(service) { $0.status == .ready }
    try await service.login(email: "fixture@example.test", password: "fixture-password")
    var state = try await accountState(service) { $0.model?.authenticated == true && $0.model?.pendingEffects == 0 }
    let choice = try #require(state.model?.catalogs.first)
    try await service.loadCatalog(index: choice.index, expectedRevision: try #require(state.model?.revision))
    state = try await accountState(service) { $0.model?.catalog?.status == .ready }
    #expect(state.model?.catalog?.items.first?.id == "account-title")
    try await service.loadDetails(index: 0, expectedRevision: try #require(state.model?.revision))
    state = try await accountState(service) { $0.model?.details?.status == .ready }
    try await service.setDetailsLibrary(true, expectedRevision: try #require(state.model?.revision))
    state = try await accountState(service) { $0.model?.sync?.state == .reauthenticationRequired }
    #expect(state.model?.sync?.pendingTitles == 1)
    return (root, namespace, storage, configuration, service, state)
}

private func accountState(_ service: NativeCoreService, matching predicate: (CoreSnapshot) -> Bool) async throws -> CoreSnapshot {
    for _ in 0..<400 {
        let state = try await service.snapshot()
        if predicate(state) { return state }
        try await Task.sleep(for: .milliseconds(10))
    }
    throw NativeCoreService.Failure.requestTimeout
}

actor AccountFixtureProbe {
    nonisolated struct Credentials: Decodable { let password: String }
    nonisolated struct Upload: Decodable, Sendable {
        nonisolated struct Change: Decodable, Sendable {
            let id: String
            enum CodingKeys: String, CodingKey { case id = "_id" }
        }
        let authKey: String
        let changes: [Change]
    }
    let recovery: Bool
    let rejectFirstRefresh: Bool
    let holdFirstRefresh: Bool
    private(set) var closedRefreshes = 0
    func refreshDisconnected() { closedRefreshes += 1 }
    var shouldHoldRefresh: Bool { holdFirstRefresh && loginCount == 2 }
    private(set) var password: String?
    private(set) var loginCount = 0
    private(set) var uploads: [Upload] = []
    init(recovery: Bool, rejectFirstRefresh: Bool, holdFirstRefresh: Bool) {
        self.recovery = recovery; self.rejectFirstRefresh = rejectFirstRefresh; self.holdFirstRefresh = holdFirstRefresh
    }
    func login(_ data: Data) -> (token: String, user: String) {
        password = try? JSONDecoder().decode(Credentials.self, from: data).password
        loginCount += 1
        return (loginCount == 1 ? "fixture-token" : "replacement-token",
                rejectFirstRefresh && loginCount == 2 ? "other-user" : "fixture-user")
    }
    func upload(_ data: Data) -> Bool {
        guard let upload = try? JSONDecoder().decode(Upload.self, from: data) else { return false }
        uploads.append(upload)
        return upload.authKey == "replacement-token"
    }
}

nonisolated final class AccountHTTPFixture: Sendable {
    let listener: NWListener
    let port: UInt16
    let probe: AccountFixtureProbe
    init(listener: NWListener, port: UInt16, probe: AccountFixtureProbe) {
        self.listener = listener; self.port = port; self.probe = probe
    }
    static func start(recovery: Bool = false, rejectFirstRefresh: Bool = false, holdFirstRefresh: Bool = false) async throws -> AccountHTTPFixture {
        let parameters = NWParameters.tcp
        parameters.requiredLocalEndpoint = .hostPort(host: "127.0.0.1", port: .any)
        let listener = try NWListener(using: parameters)
        let probe = AccountFixtureProbe(recovery: recovery, rejectFirstRefresh: rejectFirstRefresh, holdFirstRefresh: holdFirstRefresh)
        listener.newConnectionHandler = { connection in
            connection.start(queue: .global())
            receive(connection, bytes: Data(), probe: probe)
        }
        let port: UInt16 = try await withCheckedThrowingContinuation { continuation in
            listener.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    listener.stateUpdateHandler = nil
                    continuation.resume(returning: listener.port!.rawValue)
                case .failed(let error):
                    listener.stateUpdateHandler = nil
                    listener.cancel()
                    continuation.resume(throwing: error)
                default: break
                }
            }
            listener.start(queue: .global())
        }
        return AccountHTTPFixture(listener: listener, port: port, probe: probe)
    }
    private static func receive(_ connection: NWConnection, bytes: Data, probe: AccountFixtureProbe) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 4096) { chunk, _, complete, error in
            let bytes = bytes + (chunk ?? Data())
            guard bytes.count <= 16384, error == nil else { connection.cancel(); return }
            guard let delimiter = bytes.range(of: Data("\r\n\r\n".utf8)) else {
                if complete { connection.cancel() } else { receive(connection, bytes: bytes, probe: probe) }
                return
            }
            let headers = String(decoding: bytes[..<delimiter.lowerBound], as: UTF8.self)
            let length = headers.lowercased().components(separatedBy: "\r\n")
                .first(where: { $0.hasPrefix("content-length: ") }).flatMap { Int($0.dropFirst(16)) } ?? 0
            guard bytes.count >= delimiter.upperBound + length else {
                if complete { connection.cancel() } else { receive(connection, bytes: bytes, probe: probe) }
                return
            }
            Task {
                let body: String
                var status = "200 OK"
                if headers.hasPrefix("POST /api/login ") {
                    let token = await probe.login(Data(bytes[delimiter.upperBound..<(delimiter.upperBound + length)]))
                    if await probe.shouldHoldRefresh {
                        connection.receive(minimumIncompleteLength: 1, maximumLength: 1) { _, _, complete, error in
                            if complete || error != nil { Task { await probe.refreshDisconnected() } }
                            connection.cancel()
                        }
                        return
                    }
                    body = #"{"result":{"authKey":"fixture-token","user":{"_id":"fixture-user","email":"fixture@example.test","lastModified":"2026-01-01T00:00:00Z","dateRegistered":"2026-01-01T00:00:00Z","premium_expire":null,"gdpr_consent":{"tos":true,"privacy":true,"marketing":false,"from":null}}}}"#.replacingOccurrences(of: "fixture-token", with: token.token)
                        .replacingOccurrences(of: "fixture-user", with: token.user)
                } else if headers.contains("/addonCollectionGet ") {
                    if await probe.recovery {
                        body = #"{"result":{"addons":[{"transportUrl":"https://api.strem.io/fixture/manifest.json","manifest":{"id":"org.cue.account-fixture","version":"1.0.0","name":"Account fixture","description":"Owned fixture","resources":["catalog","meta"],"types":["movie"],"catalogs":[{"id":"owned","type":"movie","name":"Owned"}]}}],"lastModified":"2026-01-01T00:00:00Z"}}"#
                    } else {
                        body = #"{"result":{"addons":[],"lastModified":"2026-01-01T00:00:00Z"}}"#
                    }
                } else if headers.hasPrefix("GET /fixture/catalog/movie/owned.json ") {
                    body = #"{"metas":[{"id":"account-title","type":"movie","name":"Owned account title"}]}"#
                } else if headers.hasPrefix("GET /fixture/meta/movie/account-title.json ") {
                    body = #"{"meta":{"id":"account-title","type":"movie","name":"Owned account title"}}"#
                } else if headers.contains("/datastorePut ") {
                    if await probe.upload(Data(bytes[delimiter.upperBound..<(delimiter.upperBound + length)])) {
                        body = #"{"result":{"success":true}}"#
                    } else { status = "401 Unauthorized"; body = "{}" }
                } else if headers.contains("/datastore") { body = #"{"result":[]}"# }
                else { body = #"{"error":{"code":1,"message":"Unexpected fixture request"}}"# }
                let response = "HTTP/1.1 \(status)\r\nContent-Length: \(body.utf8.count)\r\nConnection: close\r\n\r\n\(body)"
                connection.send(content: Data(response.utf8), completion: .contentProcessed { _ in connection.cancel() })
            }
        }
    }
}
