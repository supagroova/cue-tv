import Foundation

struct YouTubeVideo: Equatable, Sendable {
    let id: String
    init?(url: URL) {
        guard let parts = URLComponents(url: url, resolvingAgainstBaseURL: false),
              parts.scheme == "https", parts.user == nil, parts.password == nil,
              parts.port == nil || parts.port == 443,
              let host = parts.host?.lowercased() else { return nil }
        let path = parts.path.split(separator: "/").map(String.init)
        let value: String?
        if host == "youtu.be", path.count == 1 { value = path[0] }
        else if ["youtube.com", "www.youtube.com", "m.youtube.com"].contains(host) {
            if path == ["watch"] {
                let ids = (parts.queryItems ?? []).filter { $0.name == "v" }
                value = ids.count == 1 ? ids[0].value : nil
            } else if path.count == 2, ["shorts", "live", "embed"].contains(path[0]) { value = path[1] }
            else { value = nil }
        } else { value = nil }
        guard let value, value.utf8.count == 11,
              value.utf8.allSatisfy({ (65...90).contains($0) || (97...122).contains($0) || (48...57).contains($0) || $0 == 45 || $0 == 95 }) else { return nil }
        id = value
    }
}

enum YouTubeHandoffError: LocalizedError, Sendable {
    case chooseVideo, rejected(String)
    var errorDescription: String? {
        switch self {
        case .chooseVideo: "Choose a YouTube video first, then tap the TV button."
        case .rejected(let message): message
        }
    }
}

actor YouTubePlaybackClient {
    private let connection = TVConnection()
    func open(_ video: YouTubeVideo, target: TVTarget, token: String) async throws {
        guard let base = target.baseURL, token.count == 64, token.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) }) else { throw TVRemoteError.pairingRequired }
        if !(await connection.isAvailable(target)) {
            guard try await connection.wake(target, token: token) == .awake else { throw TVRemoteError.pairingRequired }
        }
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = 30; configuration.timeoutIntervalForResource = 35
        configuration.waitsForConnectivity = false
        let session = URLSession(configuration: configuration, delegate: NoTVRedirects(), delegateQueue: nil)
        defer { session.invalidateAndCancel() }
        var request = URLRequest(url: base.appendingPathComponent("api/youtube/open"))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("Bearer " + token, forHTTPHeaderField: "Authorization")
        request.httpBody = try JSONEncoder().encode(["id": video.id])
        let (data, response) = try await session.data(for: request)
        try Self.validateReply(data, status: (response as? HTTPURLResponse)?.statusCode ?? 0)
    }
    static func validateReply(_ data: Data, status: Int) throws {
        struct Reply: Decodable { let ok: Bool?; let error: String? }
        if status == 401 { throw TVRemoteError.pairingRequired }
        guard data.count <= 64 * 1024 else { throw YouTubeHandoffError.rejected("The TV returned an unexpected response. Try again.") }
        let reply = try? JSONDecoder().decode(Reply.self, from: data)
        guard status == 200, reply?.ok == true else {
            throw YouTubeHandoffError.rejected(String((reply?.error ?? "Couldn’t open the video on the TV. Try again.").prefix(300)))
        }
    }
}

private final class NoTVRedirects: NSObject, URLSessionTaskDelegate {
    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse,
                    newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) {
        completionHandler(nil)
    }
}
