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
    struct CatalogChoice: Decodable, Sendable { let searchable: Bool; let index: Int; let name: String; let kind: String }
    struct CatalogItem: Decodable, Sendable { let id: String; let name: String; let kind: String }
    struct Catalog: Decodable, Sendable { let status: Addon.Status; let items: [CatalogItem]; let hasMore: Bool; let offset: Int }
    struct Video: Decodable, Sendable { let id: String; let title: String; let season: UInt32?; let episode: UInt32? }
    struct Details: Decodable, Sendable { let status: Addon.Status; let id: String; let kind: String; let name: String?; let description: String?; let videos: [Video]; let videoTotal: Int; let videoOffset: Int; let seasons: [UInt32]; let selectedSeason: UInt32? }
    struct Source: Decodable, Sendable {
        enum Kind: String, Decodable, Sendable { case torrent, http, unsupported }
        let index: Int; let kind: Kind; let name: String?; let description: String?; let sizeBytes: UInt64?
    }
    struct Sources: Decodable, Sendable { let status: Addon.Status; let loading: Bool; let videoId: String; let items: [Source]; let offset: Int; let total: Int }
    struct Model: Decodable, Sendable {
        let authenticated: Bool
        let revision: UInt64
        let stopped: Bool
        let interfaceLanguage: String
        let catalogs: [CatalogChoice]
        let catalogChoicesOffset: Int
        let catalogTotal: Int
        let sources: Sources?
        let details: Details?
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

nonisolated struct CoreTorrentRequest: Decodable, Sendable {
    let metaId: String
    let videoId: String
    let kind: String
    let magnet: String
    let fileIndex: UInt16?
    let fileMustInclude: [String]
}

nonisolated private struct CoreTorrentReply: Decodable, Sendable {
    let version: Int
    let session: UInt64
    let ticket: UInt64
    let request: CoreTorrentRequest?
    let error: String?
}

nonisolated protocol CoreService: Sendable {
    func start() async throws
    func accountIdentity() async throws -> String?
    func login(email: String, password: String) async throws
    func logout() async throws
    func snapshot() async throws -> CoreSnapshot
    func setInterfaceLanguage(_ language: String) async throws
    func loadAddon(_ manifestURL: String) async throws
    func installAddon(expectedRevision: UInt64) async throws
    func selectDetailsSeason(_ season: UInt32?, expectedRevision: UInt64) async throws
    func showDetailsVideos(offset: Int, expectedRevision: UInt64) async throws
    func loadSources(videoIndex: Int?, expectedRevision: UInt64) async throws
    func showSources(offset: Int, expectedRevision: UInt64) async throws
    func loadDetails(index: Int, expectedRevision: UInt64) async throws
    func loadCatalog(index: Int, expectedRevision: UInt64) async throws
    func searchCatalog(index: Int, query: String, expectedRevision: UInt64) async throws
    func showCatalogChoices(offset: Int, expectedRevision: UInt64) async throws
    func showCatalogItems(offset: Int, expectedRevision: UInt64) async throws
    func prepareTorrent(index: Int, expectedRevision: UInt64) async throws -> CoreTorrentRequest
    func stop() async throws
}

/// Serializes native admission off the UI actor; snapshots exclude account secrets.
actor NativeCoreService: CoreService {
    enum Failure: Error, Equatable { case unavailable, stopped, invalidRequest, busy, malformedSnapshot, shutdownTimeout, requestTimeout
        case requestFailed(String) }
    private let persistence: CorePersistenceConfiguration?
    enum Startup: Sendable { case guest, newAccount, restoreAccount(String) }
    private let startup: Startup
    /// Account restoration requires an identity from protected app storage.
    /// A missing/mismatched account fails rather than falling back to guest.
    init(persistence: CorePersistenceConfiguration? = nil, startup: Startup = .guest) {
        self.persistence = persistence
        self.startup = startup
    }
    nonisolated static func guest() async throws -> NativeCoreService {
        let configuration = try await CoreStorage.shared.prepare()
        return NativeCoreService(persistence: configuration)
    }
    nonisolated static func selected(selection: CoreAccountSelection = .shared, accountRoot: URL? = nil) async throws -> NativeCoreService {
        guard let account = try await selection.load() else { return try await guest() }
        let configuration = try await CoreStorage.account(id: account.namespace, root: accountRoot).prepareExistingAccount()
        return NativeCoreService(persistence: configuration, startup: .restoreAccount(account.userID))
    }
    private var retiringAccount = false
    private var handle: UInt64?
    private var retirement: Task<Void, any Error>?

    func start() throws {
        guard !retiringAccount else { throw Failure.stopped }
        guard retirement == nil else { throw Failure.busy }
        guard handle == nil else { return }
        if case .newAccount = startup, persistence == nil { throw Failure.invalidRequest }
        if case .restoreAccount(let expectedUser) = startup {
            guard persistence != nil, !expectedUser.isEmpty, expectedUser.utf8.count <= 512,
                  !expectedUser.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) })
            else { throw Failure.invalidRequest }
        }
        guard cue_core_abi_version() == 1 else { throw Failure.unavailable }
        let created: UInt64
        if let persistence {
            let path = Data(persistence.directory.path.utf8)
            created = path.withUnsafeBytes { pathBytes in
                persistence.key.withUnsafeBytes { keyBytes in
                    if case .newAccount = startup {
                        return cue_core_create_account(pathBytes.bindMemory(to: UInt8.self).baseAddress, pathBytes.count,
                            keyBytes.bindMemory(to: UInt8.self).baseAddress, keyBytes.count)
                    }
                    if case .restoreAccount(let expectedUser) = startup {
                        return Data(expectedUser.utf8).withUnsafeBytes { userBytes in
                            cue_core_restore_account(pathBytes.bindMemory(to: UInt8.self).baseAddress, pathBytes.count,
                                keyBytes.bindMemory(to: UInt8.self).baseAddress, keyBytes.count,
                                userBytes.bindMemory(to: UInt8.self).baseAddress, userBytes.count)
                        }
                    }
                    return cue_core_create_persistent(pathBytes.bindMemory(to: UInt8.self).baseAddress, pathBytes.count,
                                               keyBytes.bindMemory(to: UInt8.self).baseAddress, keyBytes.count)
                }
            }
        } else { created = cue_core_create() }
        guard created != 0 else { throw Failure.unavailable }
        handle = created
    }

    /// Private account-selection input. Never present this value in diagnostics.
    func accountIdentity() throws -> String? {
        guard let handle, retirement == nil else { throw Failure.stopped }
        var bytes = [UInt8](repeating: 0, count: 512)
        let size = bytes.withUnsafeMutableBufferPointer { cue_core_account_identity(handle, $0.baseAddress, $0.count) }
        if size == 0 { return nil }
        guard size > 0 else { throw Failure.stopped }
        guard size <= bytes.count, let identity = String(bytes: bytes.prefix(size), encoding: .utf8),
              !identity.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) })
        else { throw Failure.malformedSnapshot }
        return identity
    }

    /// Returns after queue admission; callers observe authenticated state/error separately.
    func login(email: String, password: String) throws {
        guard case .newAccount = startup else { throw Failure.invalidRequest }
        guard let handle, retirement == nil else { throw Failure.stopped }
        let emailBytes = Data(email.utf8)
        let passwordBytes = Data(password.utf8)
        let result = emailBytes.withUnsafeBytes { email in
            passwordBytes.withUnsafeBytes { password in
                cue_core_login(handle, email.bindMemory(to: UInt8.self).baseAddress, email.count,
                               password.bindMemory(to: UInt8.self).baseAddress, password.count)
            }
        }
        switch result {
        case 0: return
        case -2: throw Failure.invalidRequest
        case -3: throw Failure.busy
        default: throw Failure.stopped
        }
    }

    func logout() throws { try dispatch(["version": 1, "action": "logout"]) }

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

    func selectDetailsSeason(_ season: UInt32?, expectedRevision: UInt64) throws {
        try dispatch(["version": 1, "action": "details_season", "season": season.map { $0 as Any } ?? NSNull(), "revision": expectedRevision])
    }

    func showDetailsVideos(offset: Int, expectedRevision: UInt64) throws {
        guard offset >= 0 else { throw Failure.invalidRequest }
        try dispatch(["version": 1, "action": "details_videos", "offset": offset, "revision": expectedRevision])
    }

    func loadSources(videoIndex: Int?, expectedRevision: UInt64) throws {
        if let videoIndex, !(0..<24).contains(videoIndex) { throw Failure.invalidRequest }
        try dispatch(["version": 1, "action": "load_sources", "index": videoIndex.map { $0 as Any } ?? NSNull(), "revision": expectedRevision])
    }

    func showSources(offset: Int, expectedRevision: UInt64) throws {
        guard offset >= 0 else { throw Failure.invalidRequest }
        try dispatch(["version": 1, "action": "source_window", "offset": offset, "revision": expectedRevision])
    }

    func loadDetails(index: Int, expectedRevision: UInt64) throws {
        guard index >= 0 && index < 24 else { throw Failure.invalidRequest }
        try dispatch(["version": 1, "action": "load_details", "index": index, "revision": expectedRevision])
    }

    func loadCatalog(index: Int, expectedRevision: UInt64) throws {
        guard index >= 0 else { throw Failure.invalidRequest }
        try dispatch(["version": 1, "action": "load_catalog", "index": index, "revision": expectedRevision])
    }

    func searchCatalog(index: Int, query: String, expectedRevision: UInt64) throws {
        guard index >= 0, !query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              query.utf8.count <= 256 else { throw Failure.invalidRequest }
        try dispatch(["version": 1, "action": "load_catalog", "index": index, "query": query, "revision": expectedRevision])
    }

    func showCatalogChoices(offset: Int, expectedRevision: UInt64) throws {
        guard offset >= 0 else { throw Failure.invalidRequest }
        try dispatch(["version": 1, "action": "catalog_choices", "offset": offset, "revision": expectedRevision])
    }

    func showCatalogItems(offset: Int, expectedRevision: UInt64) throws {
        guard offset >= 0 else { throw Failure.invalidRequest }
        try dispatch(["version": 1, "action": "catalog_items", "offset": offset, "revision": expectedRevision])
    }

    func prepareTorrent(index: Int, expectedRevision: UInt64) async throws -> CoreTorrentRequest {
        try Task.checkCancellation()
        guard index >= 0 else { throw Failure.invalidRequest }
        guard let handle, retirement == nil else { throw Failure.stopped }
        let ticket = cue_core_request_torrent(handle, index, expectedRevision)
        guard ticket != 0 else { throw Failure.busy }
        defer { _ = cue_core_discard_torrent(handle, ticket) }
        let deadline = ContinuousClock.now.advanced(by: .seconds(5))
        var bytes = [UInt8](repeating: 0, count: 256 * 1024)
        while true {
            try Task.checkCancellation()
            guard self.handle == handle, retirement == nil else { throw Failure.stopped }
            let length = bytes.withUnsafeMutableBufferPointer { cue_core_take_torrent(handle, ticket, $0.baseAddress, $0.count) }
            guard length >= 0 else { throw Failure.unavailable }
            if length > 0 {
                guard length <= bytes.count else { throw Failure.malformedSnapshot }
                let decoder = JSONDecoder()
                decoder.keyDecodingStrategy = .convertFromSnakeCase
                let reply = try decoder.decode(CoreTorrentReply.self, from: Data(bytes.prefix(length)))
                guard reply.version == 1, reply.session == handle, reply.ticket == ticket else { throw Failure.malformedSnapshot }
                if let error = reply.error { throw Failure.requestFailed(error) }
                guard let request = reply.request else { throw Failure.malformedSnapshot }
                return request
            }
            guard ContinuousClock.now < deadline else { throw Failure.requestTimeout }
            try await Task.sleep(for: .milliseconds(10))
        }
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

    /// Permanently retires this owner before deleting its dedicated account storage.
    /// Failed shutdown retains the owner and storage so cleanup can be retried.
    func discardAccountStorage(_ storage: CoreStorage) async throws {
        guard let persistence else { throw Failure.invalidRequest }
        if case .guest = startup { throw Failure.invalidRequest }
        retiringAccount = true
        try await stop()
        try await storage.removeAccount(expectedDirectory: persistence.directory)
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
