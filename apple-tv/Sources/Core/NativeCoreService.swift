import Foundation
import CueCore

nonisolated struct CoreSnapshot: Decodable, Sendable {
    enum Status: String, Decodable, Sendable { case starting, ready, stopped }
    struct Addon: Decodable, Sendable {
        enum Status: String, Decodable, Sendable { case loading, ready, failed }
        let status: Status
        let installed: Bool
        let id: String?
        let name: String?
    }
    struct CatalogChoice: Decodable, Sendable { let index: Int; let name: String; let kind: String }
    struct CatalogItem: Decodable, Sendable { let id: String; let name: String; let kind: String }
    struct Catalog: Decodable, Sendable { let status: Addon.Status; let items: [CatalogItem]; let hasMore: Bool }
    struct Model: Decodable, Sendable {
        let revision: UInt64
        let stopped: Bool
        let interfaceLanguage: String
        let catalogs: [CatalogChoice]
        let catalogTotal: Int
        let catalog: Catalog?
        let addonCount: Int
        let addon: Addon?
        let pendingEffects: Int
        let error: String?
    }
    let version: Int
    let session: UInt64
    let status: Status
    let model: Model?
    let error: String?
}

nonisolated protocol CoreService: Sendable {
    func start() async throws
    func snapshot() async throws -> CoreSnapshot
    func setInterfaceLanguage(_ language: String) async throws
    func loadAddon(_ manifestURL: String) async throws
    func installAddon(expectedRevision: UInt64) async throws
    func loadCatalog(index: Int, expectedRevision: UInt64) async throws
    func stop() async throws
}

/// Serializes native admission off the UI actor. No account data crosses this initial service.
actor NativeCoreService: CoreService {
    enum Failure: Error, Equatable { case unavailable, stopped, invalidRequest, busy, malformedSnapshot, shutdownTimeout }
    private let persistence: CorePersistenceConfiguration?
    init(persistence: CorePersistenceConfiguration? = nil) { self.persistence = persistence }
    nonisolated static func guest() async throws -> NativeCoreService {
        let configuration = try await CoreStorage.shared.prepare()
        return NativeCoreService(persistence: configuration)
    }
    private var handle: UInt64?
    private var retirement: Task<Void, any Error>?

    func start() throws {
        guard retirement == nil else { throw Failure.busy }
        guard handle == nil else { return }
        guard cue_core_abi_version() == 1 else { throw Failure.unavailable }
        let created: UInt64
        if let persistence {
            let path = Data(persistence.directory.path.utf8)
            created = path.withUnsafeBytes { pathBytes in
                persistence.key.withUnsafeBytes { keyBytes in
                    cue_core_create_persistent(pathBytes.bindMemory(to: UInt8.self).baseAddress, pathBytes.count,
                                               keyBytes.bindMemory(to: UInt8.self).baseAddress, keyBytes.count)
                }
            }
        } else { created = cue_core_create() }
        guard created != 0 else { throw Failure.unavailable }
        handle = created
    }

    func snapshot() throws -> CoreSnapshot {
        guard let handle, retirement == nil else { throw Failure.stopped }
        var bytes = [UInt8](repeating: 0, count: 256 * 1024)
        let length = bytes.withUnsafeMutableBufferPointer { cue_core_snapshot(handle, $0.baseAddress, $0.count) }
        guard length > 0, length <= bytes.count else { throw Failure.malformedSnapshot }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        guard let value = try? decoder.decode(CoreSnapshot.self, from: Data(bytes.prefix(length))),
              value.version == 1, value.session == handle else { throw Failure.malformedSnapshot }
        return value
    }

    func setInterfaceLanguage(_ language: String) throws {
        try dispatch(["version": 1, "action": "set_language", "language": language])
    }

    func loadAddon(_ manifestURL: String) throws {
        try dispatch(["version": 1, "action": "load_addon", "url": manifestURL])
    }

    func installAddon(expectedRevision: UInt64) throws {
        try dispatch(["version": 1, "action": "install_addon", "revision": expectedRevision])
    }

    func loadCatalog(index: Int, expectedRevision: UInt64) throws {
        guard index >= 0 else { throw Failure.invalidRequest }
        try dispatch(["version": 1, "action": "load_catalog", "index": index, "revision": expectedRevision])
    }

    private func dispatch(_ command: [String: Any]) throws {
        guard let handle, retirement == nil else { throw Failure.stopped }
        let bytes = try JSONSerialization.data(withJSONObject: command)
        let result = bytes.withUnsafeBytes { cue_core_dispatch(handle, $0.bindMemory(to: UInt8.self).baseAddress, $0.count) }
        switch result {
        case 0: return
        case -2: throw Failure.invalidRequest
        case -3: throw Failure.busy
        default: throw Failure.stopped
        }
    }

    func stop() async throws {
        guard let handle else { return }
        let task: Task<Void, any Error>
        if let retirement { task = retirement }
        else {
            task = Task.detached { try await Self.retire(handle) }
            retirement = task
        }
        do {
            try await task.value
            // Actor reentrancy: a second stop may resume after a new start.
            if self.handle == handle { self.handle = nil; retirement = nil }
        } catch {
            if self.handle == handle { retirement = nil }
            throw error
        }
    }

    private nonisolated static func retire(_ handle: UInt64) async throws {
        _ = cue_core_stop(handle)
        let deadline = ContinuousClock.now.advanced(by: .seconds(5))
        while cue_core_stop_status(handle) == 0 {
            guard ContinuousClock.now < deadline else { throw Failure.shutdownTimeout }
            try await Task.sleep(for: .milliseconds(10))
        }
        guard cue_core_release(handle) == 1 else { throw Failure.unavailable }
    }

    deinit {
        if let handle, retirement == nil {
            Task.detached { try? await Self.retire(handle) }
        }
    }
}
