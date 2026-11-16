import Foundation

struct TVTarget: Codable, Hashable, Sendable {
    var name: String
    var host: String
    var macAddress: String

    static func initial() -> Self {
        if let saved = UserDefaults.standard.data(forKey: "tv.target"), let target = try? JSONDecoder().decode(Self.self, from: saved), target.isValid { return target }
        if let url = Bundle.main.url(forResource: "TVConfig", withExtension: "plist"),
           let data = try? Data(contentsOf: url), let config = try? PropertyListSerialization.propertyList(from: data, format: nil) as? [String: String] {
            return Self(name: config["Name"] ?? "TV", host: config["Host"] ?? "", macAddress: config["MACAddress"] ?? "")
        }
        return Self(name: "TV", host: "", macAddress: "")
    }

    var baseURL: URL? {
        guard isValid else { return nil }
        var c = URLComponents(); c.scheme = "http"; c.host = host; c.port = 5400
        return c.url
    }
    var credentialID: String { macAddress.lowercased().replacingOccurrences(of: "-", with: ":") }
    var isValid: Bool { Self.isPrivateIPv4(host) && (try? WakePacket.make(macAddress: macAddress)) != nil && !name.trimmingCharacters(in: .whitespaces).isEmpty }

    static func isPrivateIPv4(_ host: String) -> Bool {
        let pieces = host.split(separator: ".", omittingEmptySubsequences: false)
        guard pieces.count == 4 else { return false }
        let octets = pieces.compactMap { p -> Int? in
            guard !p.isEmpty, p.allSatisfy({ $0.isASCII && $0.isNumber }), let v = Int(p), (0...255).contains(v), String(v) == p else { return nil }
            return v
        }
        guard octets.count == 4 else { return false }
        return octets[0] == 10 || (octets[0] == 172 && (16...31).contains(octets[1])) || (octets[0] == 192 && octets[1] == 168)
    }
}

enum WakePacket {
    static func make(macAddress: String) throws -> Data {
        let fields = macAddress.replacingOccurrences(of: "-", with: ":").split(separator: ":", omittingEmptySubsequences: false)
        guard fields.count == 6 else { throw TVRemoteError.invalidTarget }
        let mac = fields.compactMap { field -> UInt8? in
            guard field.count == 2 else { return nil }
            return UInt8(field, radix: 16)
        }
        guard mac.count == 6, mac.contains(where: { $0 != 0 }), mac[0] & 1 == 0 else { throw TVRemoteError.invalidTarget }
        var packet = Data(repeating: 0xff, count: 6)
        for _ in 0..<16 { packet.append(contentsOf: mac) }
        return packet
    }
}

enum TVRemoteError: LocalizedError, Sendable {
    case invalidTarget, notReachable, localNetworkDenied, pairingRequired
    var errorDescription: String? {
        switch self {
        case .invalidTarget: "Check the TV address and network address in Settings."
        case .notReachable: "The TV hasn’t answered yet. Check that your iPhone is on the home Wi-Fi and try again."
        case .localNetworkDenied: "Allow Local Network access for TV in iPhone Settings, then try again."
        case .pairingRequired: "Pair this iPhone with the TV using the code on its screen."
        }
    }
}
