import Foundation
import Security

/// One shared actor owns active-account selection; credentials remain in encrypted Core storage.
actor CoreAccountSelection {
    nonisolated struct Account: Codable, Equatable, Sendable {
        let namespace: UUID
        let userID: String
    }
    nonisolated private struct Record: Codable {
        let version: Int
        let account: Account
    }
    enum Failure: Error, Equatable { case invalidRecord, keychain(OSStatus) }
    static let shared = CoreAccountSelection(keyAccount: "active-account-v1")
    private let keyAccount: String
    init(keyAccount: String) { self.keyAccount = keyAccount }

    private var query: [String: Any] {
        [kSecClass as String: kSecClassGenericPassword,
         kSecAttrService as String: CoreStorage.keyService,
         kSecAttrAccount as String: keyAccount,
         kSecAttrSynchronizable as String: false]
    }
    private func validate(_ account: Account) throws {
        guard !account.userID.isEmpty, account.userID.utf8.count <= 512,
              !account.userID.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) })
        else { throw Failure.invalidRecord }
    }
    func load() throws -> Account? {
        var read = query
        read[kSecReturnData as String] = true
        read[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        let status = SecItemCopyMatching(read as CFDictionary, &result)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess else { throw Failure.keychain(status) }
        guard let data = result as? Data, data.count <= 4096,
              let record = try? JSONDecoder().decode(Record.self, from: data), record.version == 1
        else { throw Failure.invalidRecord }
        try validate(record.account)
        return record.account
    }
    /// Save only after native authentication and identity verification have succeeded.
    func save(_ account: Account) throws {
        try validate(account)
        let data = try JSONEncoder().encode(Record(version: 1, account: account))
        let attributes: [String: Any] = [kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly]
        let status = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
        if status == errSecSuccess { return }
        guard status == errSecItemNotFound else { throw Failure.keychain(status) }
        let added = SecItemAdd(query.merging(attributes) { _, new in new } as CFDictionary, nil)
        guard added == errSecSuccess else { throw Failure.keychain(added) }
    }
    /// A late sign-out from an older session must not remove a newer selection.
    @discardableResult
    func clear(ifMatching namespace: UUID) throws -> Bool {
        guard let current = try load(), current.namespace == namespace else { return false }
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else { throw Failure.keychain(status) }
        return true
    }
}
