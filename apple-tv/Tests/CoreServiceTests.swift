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
    #expect(state.model?.catalog?.items.first?.id == "tt-fixture")
    #expect(state.model?.catalog?.items.first?.name == "Simulator film")
    #expect(state.model?.catalog?.items.count == 24)
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
                let isCatalog = requestText.contains("/catalog/movie/fixture")
                let range = requestText.contains("skip=30") ? 30..<30 : requestText.contains("skip=27") ? 27..<30 : 0..<27
                let metas = range.map { index in ["id": index == 0 ? "tt-fixture" : "tt-fixture-\(index)", "name": index == 0 ? "Simulator film" : "Film \(index)", "type": "movie"] }
                let catalogBody = String(decoding: try! JSONSerialization.data(withJSONObject: ["metas": metas]), as: UTF8.self)
                let body = isCatalog
                    ? catalogBody
                    : #"{"id":"org.cue.simulator","name":"Simulator fixture","version":"1.0.0","description":"Owned fixture","resources":["catalog"],"types":["movie"],"catalogs":[{"id":"fixture","type":"movie","name":"Fixture catalogue","extra":[{"name":"skip"}]}]}"#
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
