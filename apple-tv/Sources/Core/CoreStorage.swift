import Foundation
import Security

nonisolated struct CorePersistenceConfiguration: Sendable {
    let directory: URL
    let key: Data
}

/// Filesystem and Keychain work run off the UI actor. Account storage remains separate.
actor CoreStorage {
    enum Failure: Error, Equatable {
        case unavailable, missingKey, missingState, invalidKey, unsafeDirectory
        case keychain(OSStatus)
    }
    static let keyService = "com.cue-tv.stremio-core.storage-v1"
    static let shared = CoreStorage(
        directory: FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("CueCore/guest-v1", isDirectory: true),
        keyAccount: "guest-v1"
    )
    /// Each login attempt receives a fresh opaque namespace. Reuse its UUID only
    /// after authenticated identity/active-account restoration has been verified.
    /// No email, provider user ID or token is used as a path or Keychain label.
    nonisolated static func account(id: UUID, root: URL? = nil) -> CoreStorage {
        let base = root ?? FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("CueCore/accounts-v1", isDirectory: true)
        let name = id.uuidString.lowercased()
        return CoreStorage(accountDirectory: base.appendingPathComponent(name, isDirectory: true),
                           keyAccount: "account-v1:" + name)
    }

    private let directory: URL
    private let keyAccount: String
    private let removableAccount: Bool
    init(directory: URL, keyAccount: String) {
        self.directory = directory; self.keyAccount = keyAccount; self.removableAccount = false
    }
    private init(accountDirectory: URL, keyAccount: String) {
        self.directory = accountDirectory; self.keyAccount = keyAccount; self.removableAccount = true
    }
    /// Candidate login owner may call this only before creating any native owner.
    func removeUnopenedAccount() throws { try removeAccount(expectedDirectory: directory) }
    /// Only the owning service calls this after native stop acknowledgement.
    func removeAccount(expectedDirectory: URL) throws {
        guard removableAccount, directory == expectedDirectory, directory.isFileURL else { throw Failure.unsafeDirectory }
        if FileManager.default.fileExists(atPath: directory.path) {
            let values = try directory.resourceValues(forKeys: [.isSymbolicLinkKey, .isDirectoryKey])
            guard values.isSymbolicLink != true, values.isDirectory == true else { throw Failure.unsafeDirectory }
            try FileManager.default.removeItem(at: directory)
        }
        // Keep the key if filesystem removal fails, so retained ciphertext stays recoverable.
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: Self.keyService, kSecAttrAccount as String: keyAccount]
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else { throw Failure.keychain(status) }
    }

    func prepareExistingAccount() throws -> CorePersistenceConfiguration {
        guard removableAccount else { throw Failure.unsafeDirectory }
        guard FileManager.default.fileExists(atPath: directory.appendingPathComponent("state.bin").path)
        else { throw Failure.missingState }
        return try prepare()
    }

    func prepare() throws -> CorePersistenceConfiguration {
        guard directory.isFileURL else { throw Failure.unsafeDirectory }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        guard try directory.resourceValues(forKeys: [.isSymbolicLinkKey]).isSymbolicLink != true else { throw Failure.unsafeDirectory }
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
                                   kSecAttrService as String: Self.keyService,
                                   kSecAttrAccount as String: keyAccount]
        var read = query
        read[kSecReturnData as String] = true
        read[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        let status = SecItemCopyMatching(read as CFDictionary, &result)
        if status == errSecSuccess {
            guard let key = result as? Data, key.count == 32 else { throw Failure.invalidKey }
            return CorePersistenceConfiguration(directory: directory, key: key)
        }
        guard status == errSecItemNotFound else { throw Failure.keychain(status) }
        guard !FileManager.default.fileExists(atPath: directory.appendingPathComponent("state.bin").path) else { throw Failure.missingKey }
        var key = Data(count: 32)
        let generated = key.withUnsafeMutableBytes { SecRandomCopyBytes(kSecRandomDefault, $0.count, $0.baseAddress!) }
        guard generated == errSecSuccess else { throw Failure.unavailable }
        var add = query
        add[kSecValueData as String] = key
        add[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        add[kSecAttrSynchronizable as String] = false
        let saved = SecItemAdd(add as CFDictionary, nil)
        if saved == errSecDuplicateItem {
            // Another storage actor won creation. Re-read once; never replace it.
            var existing: CFTypeRef?
            let retried = SecItemCopyMatching(read as CFDictionary, &existing)
            guard retried == errSecSuccess else { throw Failure.keychain(retried) }
            guard let existing = existing as? Data, existing.count == 32 else { throw Failure.invalidKey }
            return CorePersistenceConfiguration(directory: directory, key: existing)
        }
        guard saved == errSecSuccess else { throw Failure.keychain(saved) }
        return CorePersistenceConfiguration(directory: directory, key: key)
    }
}
