import Foundation
import Security

final class TVService {
    var connection: TVConnection?
    private let session: URLSession
    private let eventsSession: URLSession
    init(connection: TVConnection?) {
        self.connection = connection
        let config = URLSessionConfiguration.ephemeral
        config.timeoutIntervalForRequest = 20
        config.timeoutIntervalForResource = 35
        config.requestCachePolicy = .reloadIgnoringLocalCacheData
        session = URLSession(configuration: config)
        let events = URLSessionConfiguration.ephemeral
        events.timeoutIntervalForRequest = 30
        events.timeoutIntervalForResource = 0
        eventsSession = URLSession(configuration: events)
    }
    func request<T: Decodable & Sendable>(_ path: String, as: T.Type, query: String? = nil, parameters: [String: String] = [:]) async throws -> T {
        guard let connection, let base = TVAddress.normalise(connection.address), var url = URLComponents(url: base.appendingPathComponent(path), resolvingAgainstBaseURL: false) else {
            throw TVError(message: "Connect to Sitting Room TV to load your library.", status: 401)
        }
        if let query { url.queryItems = [URLQueryItem(name: "q", value: query)] }
        if !parameters.isEmpty { url.queryItems = (url.queryItems ?? []) + parameters.sorted { $0.key < $1.key }.map { URLQueryItem(name: $0.key, value: $0.value) } }
        guard let target = url.url else { throw TVError(message: "Check the TV address.", status: 400) }
        var request = URLRequest(url: target)
        request.setValue("Bearer " + connection.token, forHTTPHeaderField: "Authorization")
        return try await send(request, as: T.self)
    }
    func pair(address: String, code: String) async throws -> TVConnection {
        guard let base = TVAddress.normalise(address), code.range(of: "^[0-9]{6}$", options: .regularExpression) != nil else {
            throw TVError(message: "Enter the TV's local address and six-digit code.", status: 400)
        }
        var request = URLRequest(url: base.appendingPathComponent("api/pair"))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: ["pin": code, "name": "Living room Apple TV"])
        let paired = try await send(request, as: PairResponse.self)
        return TVConnection(address: base.absoluteString, token: paired.token)
    }
    func launch(_ app: String) async throws {
        guard ["youtube", "stremio"].contains(app) else { return }
        try await action("api/platform/apple/launch", body: ["app": app])
    }
    func confirmTitle(_ serial: Int64, opened: Bool) async throws {
        try await action("api/platform/apple/opened", body: ["serial": serial, "opened": opened])
    }
    func cueChoice(_ choice: String) async throws -> CueMedia {
        let result: CueMediaReply = try await post("api/stremio/playback/choice", body: ["id": choice])
        return result.media
    }
    func progress(_ media: CueMedia, position: Int64, duration: Int64, state: String) async throws {
        try await action("api/stremio/progress", body: ["playback": media.playback, "position": position, "duration": duration, "state": state])
    }
    private func action(_ path: String, body: [String: Any]) async throws {
        let _: Acknowledgement = try await post(path, body: body)
    }
    private func post<T: Decodable & Sendable>(_ path: String, body: [String: Any]) async throws -> T {
        guard let connection, let base = TVAddress.normalise(connection.address) else {
            throw TVError(message: "Connect to Sitting Room TV to open this app.", status: 401)
        }
        var request = URLRequest(url: base.appendingPathComponent(path))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("Bearer " + connection.token, forHTTPHeaderField: "Authorization")
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        return try await send(request, as: T.self)
    }
    /// Only while the app is in front. The Android service keeps working while tvOS suspends us.
    func events(_ onEvent: (TVEvent) -> Void) async {
        while !Task.isCancelled {
            guard let connection, let base = TVAddress.normalise(connection.address) else { return }
            var request = URLRequest(url: base.appendingPathComponent("api/events"))
            request.setValue("Bearer " + connection.token, forHTTPHeaderField: "Authorization")
            do {
                let (bytes, response) = try await eventsSession.bytes(for: request)
                guard (response as? HTTPURLResponse)?.statusCode == 200 else { return }
                for try await line in bytes.lines {
                    try Task.checkCancellation()
                    guard line.hasPrefix("data: "), let data = String(line.dropFirst(6)).data(using: .utf8),
                          let signal = try? JSONDecoder().decode(TVEvent.self, from: data) else { continue }
                    onEvent(signal)
                }
            } catch { if Task.isCancelled { return } }
            do { try await Task.sleep(for: .seconds(1)) } catch { return }
        }
    }
    private func send<T: Decodable & Sendable>(_ request: URLRequest, as: T.Type) async throws -> T {
        let (data, response) = try await session.data(for: request)
        let status = (response as? HTTPURLResponse)?.statusCode ?? 0
        guard (200..<300).contains(status) else {
            let message = try? JSONDecoder().decode(APIMessage.self, from: data)
            throw TVError(message: message?.error ?? "Couldn't reach Sitting Room TV. Check it is switched on.", status: status)
        }
        return try JSONDecoder().decode(T.self, from: data)
    }
}

enum ConnectionKeychain {
    private static let service = "com.rupertlaycock.couchtv.appletv.connection"
    static func load() -> TVConnection? {
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service,
                                  kSecAttrAccount as String: "television", kSecReturnData as String: true, kSecMatchLimit as String: kSecMatchLimitOne]
        var result: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess, let data = result as? Data else { return nil }
        return try? JSONDecoder().decode(TVConnection.self, from: data)
    }
    static func save(_ connection: TVConnection) throws {
        let data = try JSONEncoder().encode(connection)
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service, kSecAttrAccount as String: "television"]
        let status = SecItemUpdate(query as CFDictionary, [kSecValueData as String: data] as CFDictionary)
        if status == errSecItemNotFound {
            var add = query; add[kSecValueData as String] = data; add[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
            guard SecItemAdd(add as CFDictionary, nil) == errSecSuccess else { throw TVError(message: "Couldn't save the connection. Try connecting again.", status: 500) }
        } else if status != errSecSuccess { throw TVError(message: "Couldn't save the connection.", status: 500) }
    }
}
