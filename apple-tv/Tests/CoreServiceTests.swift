import Foundation
import Testing
import Network
import Security
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
    #expect(try await service.accountIdentity() == nil)
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
    await #expect(throws: NativeCoreService.Failure.stopped) { _ = try await service.accountIdentity() }
    try await service.start()
    #expect(try await service.snapshot().session != state.session)
    try await service.stop()
}

@Test func swiftCoreLoadsManifestOverSimulatorHTTP() async throws {
    let fixture = try await CoreHTTPFixture.start()
    defer { fixture.listener.cancel() }
    let service = NativeCoreService()
    try await service.start()
    try await service.loadAddon("http://127.0.0.1:\(fixture.port)/private-config/manifest.json")
    var state = try await service.snapshot()
    for _ in 0..<200 {
        if state.model?.addon?.status == .ready { break }
        try await Task.sleep(for: .milliseconds(10))
        state = try await service.snapshot()
    }
    #expect(state.model?.addon?.status == .ready)
    #expect(state.model?.addon?.id == "org.cue.simulator")
    #expect(state.model?.addon?.name == "Simulator fixture")
    let originalCount = state.model?.addonCount
    try await service.installAddon(expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.addon?.installed == true && state.model?.pendingEffects == 0 { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.addon?.installed == true)
    #expect(state.model?.addonCount == originalCount.map { $0 + 1 })
    let beforePaging = try #require(state.model?.revision)
    try await service.showCatalogChoices(offset: 0, expectedRevision: beforePaging)
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.revision != beforePaging { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.catalogChoicesOffset == 0)
    let choice = try #require(state.model?.catalogs.first { $0.name == "Fixture catalogue" })
    try await service.loadCatalog(index: choice.index, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.catalog?.status == .ready { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.catalog?.items.first?.id == "cue-fixture")
    #expect(state.model?.catalog?.items.first?.name == "Simulator film")
    #expect(state.model?.catalog?.items.count == 24)
    #expect(choice.searchable)
    try await service.searchCatalog(index: choice.index, query: "needle", expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.catalog?.items.first?.name == "Search result" { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.catalog?.items.first?.name == "Search result")
    try await service.loadDetails(index: 0, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.details?.status == .ready { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.details?.id == "cue-fixture")
    #expect(state.model?.details?.name == "Detailed simulator film")
    #expect(state.model?.details?.description == "Owned title details")
    #expect(state.model?.details?.videoTotal == 30)
    #expect(state.model?.details?.videos.count == 24)
    try await service.showDetailsVideos(offset: 24, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.details?.videoOffset == 24 { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.details?.videos.count == 6)
    #expect(state.model?.details?.videos.last?.id == "cue-fixture:30")
    try await service.loadSources(videoIndex: 5, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.sources?.status == .ready { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.sources?.videoId == "cue-fixture:30")
    #expect(state.model?.sources?.total == 30)
    #expect(state.model?.sources?.items.count == 24)
    #expect(state.model?.sources?.items.first?.kind == .torrent)
    #expect(state.model?.sources?.items.first?.sizeBytes == nil)
    try await service.showSources(offset: 24, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.sources?.offset == 24 { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.sources?.items.count == 6)
    #expect(state.model?.sources?.items.last?.name == "Source 29")
    let handoff = try await service.prepareTorrent(index: 29, expectedRevision: try #require(state.model?.revision))
    #expect(handoff.magnet.contains("0123456789abcdef0123456789abcdef01234567"))
    #expect(handoff.metaId == "cue-fixture")
    #expect(handoff.videoId == "cue-fixture:30")
    #expect(handoff.kind == "movie")
    #expect(handoff.fileIndex == nil)
    #expect(handoff.fileMustInclude.isEmpty)
    await #expect(throws: NativeCoreService.Failure.requestFailed("stale_sources")) {
        try await service.prepareTorrent(index: 29, expectedRevision: 0)
    }
    for offset in [24, 27, 30] {
        try await service.showCatalogItems(offset: offset, expectedRevision: try #require(state.model?.revision))
        for _ in 0..<200 {
            state = try await service.snapshot()
            if state.model?.catalog?.offset == offset && state.model?.catalog?.status == .ready { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(state.model?.catalog?.items.count == (offset == 30 ? 0 : 3))
    }
    #expect(state.model?.catalog?.hasMore == false)
    #expect(state.error == nil)
    #expect(state.model?.error == nil)
    try await service.stop()
}

@Test func accountRestoreDoesNotFallBackToGuestWhenStorageIsEmpty() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: directory) }
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    let configuration = CorePersistenceConfiguration(directory: directory, key: Data(repeating: 49, count: 32))
    let account = NativeCoreService(persistence: configuration, startup: .restoreAccount("fixture-user"))
    try await account.start()
    var state = try await account.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10))
        state = try await account.snapshot()
    }
    #expect(state.status == .stopped)
    #expect(state.error == "core_start_failed")
    try await account.stop()
    // Failed account startup must release the store, so a guest can open it explicitly.
    let guest = NativeCoreService(persistence: configuration)
    try await guest.start()
    state = try await guest.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10))
        state = try await guest.snapshot()
    }
    #expect(state.status == .ready)
    #expect(state.model?.authenticated == false)
    try await guest.stop()
    let missingStore = NativeCoreService(startup: .restoreAccount("fixture-user"))
    await #expect(throws: NativeCoreService.Failure.invalidRequest) { try await missingStore.start() }
}

@Test func keychainBackedGuestSurvivesServiceRecreation() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
    let account = "test-" + UUID().uuidString
    let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
                               kSecAttrService as String: CoreStorage.keyService,
                               kSecAttrAccount as String: account]
    defer { SecItemDelete(query as CFDictionary); try? FileManager.default.removeItem(at: directory) }
    let store = CoreStorage(directory: directory, keyAccount: account)
    let first = NativeCoreService(persistence: try await store.prepare())
    try await first.start()
    try await first.setInterfaceLanguage("deu")
    var state = try await first.snapshot()
    for _ in 0..<200 {
        if state.model?.interfaceLanguage == "deu" && state.model?.pendingEffects == 0 { break }
        try await Task.sleep(for: .milliseconds(10)); state = try await first.snapshot()
    }
    #expect(state.model?.interfaceLanguage == "deu")
    #expect(state.model?.error == nil)
    try await first.stop()
    let second = NativeCoreService(persistence: try await store.prepare())
    try await second.start()
    state = try await second.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10)); state = try await second.snapshot()
    }
    #expect(state.model?.interfaceLanguage == "deu")
    try await second.stop()
    #expect(SecItemDelete(query as CFDictionary) == errSecSuccess)
    await #expect(throws: CoreStorage.Failure.missingKey) { try await store.prepare() }
    #expect(FileManager.default.fileExists(atPath: directory.appendingPathComponent("state.bin").path))
}

}


nonisolated private final class CoreHTTPFixture: Sendable {
    let listener: NWListener
    let port: UInt16
    init(listener: NWListener, port: UInt16) { self.listener = listener; self.port = port }
    static func start() async throws -> CoreHTTPFixture {
        let listener = try NWListener(using: .tcp, on: .any)
        listener.newConnectionHandler = { connection in
            connection.start(queue: .global())
            connection.receive(minimumIncompleteLength: 1, maximumLength: 4096) { request, _, _, _ in
                let requestText = request.map { String(decoding: $0, as: UTF8.self) } ?? ""
                let isMetadata = requestText.contains("/meta/movie/cue-fixture.json")
                let isCatalog = requestText.contains("/catalog/movie/fixture")
                let range = requestText.contains("skip=30") ? 30..<30 : requestText.contains("skip=27") ? 27..<30 : 0..<27
                let metas = range.map { index in ["id": index == 0 ? "cue-fixture" : "cue-fixture-\(index)", "name": index == 0 ? (requestText.contains("search=needle") ? "Search result" : "Simulator film") : "Film \(index)", "type": "movie"] }
                let catalogBody = String(decoding: try! JSONSerialization.data(withJSONObject: ["metas": metas]), as: UTF8.self)
                let videos = (1...30).map { ["id": "cue-fixture:\($0)", "title": "Edition \($0)"] }
                let metadataBody = String(decoding: try! JSONSerialization.data(withJSONObject: ["meta": ["id": "cue-fixture", "type": "movie", "name": "Detailed simulator film", "description": "Owned title details", "videos": videos]]), as: UTF8.self)
                let streams = (0..<30).map { ["infoHash": "0123456789abcdef0123456789abcdef01234567", "name": "Source \($0)"] }
                let sourceBody = String(decoding: try! JSONSerialization.data(withJSONObject: ["streams": streams]), as: UTF8.self)
                let body = requestText.contains("/stream/movie/") ? sourceBody : isCatalog
                    ? catalogBody
                    : isMetadata ? metadataBody
                    : #"{"id":"org.cue.simulator","name":"Simulator fixture","version":"1.0.0","description":"Owned fixture","resources":["catalog",{"name":"stream","types":["movie"],"idPrefixes":["cue-"]},{"name":"meta","types":["movie"],"idPrefixes":["cue-"]}],"types":["movie"],"catalogs":[{"id":"fixture","type":"movie","name":"Fixture catalogue","extra":[{"name":"skip"},{"name":"search"}]}]}"#
                let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: \(body.utf8.count)\r\nConnection: close\r\n\r\n\(body)"
                connection.send(content: Data(response.utf8), completion: .contentProcessed { _ in connection.cancel() })
            }
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
        return CoreHTTPFixture(listener: listener, port: port)
    }
}

@Test func accountNamespacesUseIndependentKeysAndState() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let firstID = UUID(), secondID = UUID()
    defer {
        for id in [firstID, secondID] {
            let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
                kSecAttrService as String: CoreStorage.keyService,
                kSecAttrAccount as String: "account-v1:" + id.uuidString.lowercased()]
            SecItemDelete(query as CFDictionary)
        }
        try? FileManager.default.removeItem(at: root)
    }
    let firstStore = CoreStorage.account(id: firstID, root: root)
    let secondStore = CoreStorage.account(id: secondID, root: root)
    let firstConfig = try await firstStore.prepare()
    let secondConfig = try await secondStore.prepare()
    #expect(firstConfig.directory != secondConfig.directory)
    #expect(firstConfig.key != secondConfig.key)
    #expect(firstConfig.directory.lastPathComponent == firstID.uuidString.lowercased())
    let first = NativeCoreService(persistence: firstConfig)
    try await first.start()
    try await first.setInterfaceLanguage("fra")
    var state = try await first.snapshot()
    for _ in 0..<200 where state.model?.interfaceLanguage != "fra" || state.model?.pendingEffects != 0 {
        try await Task.sleep(for: .milliseconds(10)); state = try await first.snapshot()
    }
    #expect(state.model?.interfaceLanguage == "fra")
    try await first.stop()
    let second = NativeCoreService(persistence: secondConfig)
    try await second.start()
    state = try await second.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10)); state = try await second.snapshot()
    }
    #expect(state.model?.interfaceLanguage == "eng")
    try await second.stop()
    let restored = try await CoreStorage.account(id: firstID, root: root).prepare()
    #expect(restored.key == firstConfig.key)
    let reopened = NativeCoreService(persistence: restored)
    try await reopened.start()
    state = try await reopened.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10)); state = try await reopened.snapshot()
    }
    #expect(state.model?.interfaceLanguage == "fra")
    try await reopened.stop()
}

@Test func freshAccountServiceValidatesCredentialsAndCannotReuseStorage() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: directory) }
    let configuration = CorePersistenceConfiguration(directory: directory, key: Data(repeating: 83, count: 32))
    let service = NativeCoreService(persistence: configuration, startup: .newAccount)
    try await service.start()
    var state = try await service.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10)); state = try await service.snapshot()
    }
    #expect(state.status == .ready)
    await #expect(throws: NativeCoreService.Failure.invalidRequest) { try await service.login(email: "", password: "synthetic-password") }
    let revision = try #require(state.model?.revision)
    try await service.logout()
    for _ in 0..<200 where state.model?.revision == revision {
        try await Task.sleep(for: .milliseconds(10)); state = try await service.snapshot()
    }
    #expect(state.model?.revision != revision)
    #expect(state.model?.authenticated == false)
    try await service.stop()
    try await service.start()
    state = try await service.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10)); state = try await service.snapshot()
    }
    #expect(state.status == .stopped)
    #expect(state.error == "core_start_failed")
    try await service.stop()
    let guest = NativeCoreService()
    try await guest.start()
    await #expect(throws: NativeCoreService.Failure.invalidRequest) { try await guest.login(email: "fixture@example.test", password: "synthetic-password") }
    try await guest.stop()
}

@Test func activeAccountSelectionRoundTripsAndIgnoresStaleSignOut() async throws {
    let label = "selection-test-" + UUID().uuidString
    let store = CoreAccountSelection(keyAccount: label)
    let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: CoreStorage.keyService, kSecAttrAccount as String: label]
    defer { SecItemDelete(query as CFDictionary) }
    #expect(try await store.load() == nil)
    let first = CoreAccountSelection.Account(namespace: UUID(), userID: "first-fixture-user")
    let second = CoreAccountSelection.Account(namespace: UUID(), userID: "second-fixture-user")
    try await store.save(first)
    var attributesQuery = query
    attributesQuery[kSecReturnAttributes as String] = true
    var attributesResult: CFTypeRef?
    #expect(SecItemCopyMatching(attributesQuery as CFDictionary, &attributesResult) == errSecSuccess)
    let attributes = try #require(attributesResult as? [String: Any])
    #expect(attributes[kSecAttrAccessible as String] as? String == kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly as String)
    #expect(attributes[kSecAttrSynchronizable as String] as? Bool != true)
    #expect(try await CoreAccountSelection(keyAccount: label).load() == first)
    try await store.save(second)
    #expect(try await store.clear(ifMatching: first.namespace) == false)
    #expect(try await store.load() == second)
    #expect(try await store.clear(ifMatching: second.namespace) == true)
    #expect(try await store.load() == nil)
    await #expect(throws: CoreAccountSelection.Failure.invalidRecord) {
        try await store.save(.init(namespace: UUID(), userID: ""))
    }
    try await store.save(first)
    let invalid = Data(#"{"version":99,"account":{"namespace":"00000000-0000-0000-0000-000000000000","userID":"fixture"}}"#.utf8)
    #expect(SecItemUpdate(query as CFDictionary, [kSecValueData as String: invalid] as CFDictionary) == errSecSuccess)
    await #expect(throws: CoreAccountSelection.Failure.invalidRecord) { try await store.load() }
}
