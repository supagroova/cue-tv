import Foundation
import Security

@MainActor
struct PairingStore {
    private let service = "com.rupertlaycock.couchtv.remote.pairing"
    private func query(account: String) -> [String: Any] {
        var value: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service, kSecAttrAccount as String: account]
        if let group = Bundle.main.object(forInfoDictionaryKey: "TVPairingAccessGroup") as? String,
           !group.contains("$("), !group.isEmpty { value[kSecAttrAccessGroup as String] = group }
        return value
    }
    private func read(account: String) -> Data? {
        var query = query(account: account)
        query[kSecReturnData as String] = true; query[kSecMatchLimit as String] = kSecMatchLimitOne
        var value: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &value) == errSecSuccess else { return nil }
        return value as? Data
    }
    private func write(_ data: Data, account: String) {
        let query = query(account: account)
        if SecItemUpdate(query as CFDictionary, [kSecValueData as String: data] as CFDictionary) == errSecItemNotFound {
            var item = query; item[kSecValueData as String] = data
            item[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
            SecItemAdd(item as CFDictionary, nil)
        }
    }
    func token(for target: TVTarget) -> String? {
        guard let data = read(account: target.credentialID) else { return nil }
        return String(data: data, encoding: .utf8)
    }
    func save(_ token: String, for target: TVTarget) {
        guard token.count == 64, token.allSatisfy({ $0.isHexDigit && $0.isASCII }), let data = token.data(using: .utf8) else { return }
        write(data, account: target.credentialID)
        saveCurrentTarget(target)
    }
    func saveCurrentTarget(_ target: TVTarget) {
        guard target.isValid, let data = try? JSONEncoder().encode(target) else { return }
        write(data, account: "current-tv-target")
    }
    func currentTarget() -> TVTarget? {
        guard let data = read(account: "current-tv-target"),
              let target = try? JSONDecoder().decode(TVTarget.self, from: data), target.isValid else { return nil }
        return target
    }
}
