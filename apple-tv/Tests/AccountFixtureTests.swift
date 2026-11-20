import Foundation
import Network
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

actor AccountFixtureProbe {
    nonisolated struct Credentials: Decodable { let password: String }
    private(set) var password: String?
    func login(_ data: Data) { password = try? JSONDecoder().decode(Credentials.self, from: data).password }
}

nonisolated final class AccountHTTPFixture: Sendable {
    let listener: NWListener
    let port: UInt16
    let probe: AccountFixtureProbe
    init(listener: NWListener, port: UInt16, probe: AccountFixtureProbe) {
        self.listener = listener; self.port = port; self.probe = probe
    }
    static func start() async throws -> AccountHTTPFixture {
        let parameters = NWParameters.tcp
        parameters.requiredLocalEndpoint = .hostPort(host: "127.0.0.1", port: .any)
        let listener = try NWListener(using: parameters)
        let probe = AccountFixtureProbe()
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
                if headers.hasPrefix("POST /api/login ") {
                    await probe.login(Data(bytes[delimiter.upperBound..<(delimiter.upperBound + length)]))
                    body = #"{"result":{"authKey":"fixture-token","user":{"_id":"fixture-user","email":"fixture@example.test","lastModified":"2026-01-01T00:00:00Z","dateRegistered":"2026-01-01T00:00:00Z","premium_expire":null,"gdpr_consent":{"tos":true,"privacy":true,"marketing":false,"from":null}}}}"#
                } else if headers.contains("/addonCollectionGet ") {
                    body = #"{"result":{"addons":[],"lastModified":"2026-01-01T00:00:00Z"}}"#
                } else if headers.contains("/datastore") { body = #"{"result":[]}"# }
                else { body = #"{"error":{"code":1,"message":"Unexpected fixture request"}}"# }
                let response = "HTTP/1.1 200 OK\r\nContent-Length: \(body.utf8.count)\r\nConnection: close\r\n\r\n\(body)"
                connection.send(content: Data(response.utf8), completion: .contentProcessed { _ in connection.cancel() })
            }
        }
    }
}
