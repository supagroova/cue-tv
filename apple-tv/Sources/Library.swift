import Foundation

nonisolated struct Title: Decodable, Identifiable, Hashable, Sendable {
    let id: String
    let type: String
    let title: String
    let poster: String?
    let background: String?
    let description: String?
    let year: String?
    let progress: Double?
    let inProgress: Bool?
    let video: String?
    var identity: String { "\(type):\(id)" }
    var posterURL: URL? { Self.imageURL(poster) }
    var backgroundURL: URL? { Self.imageURL(background) }
    static func imageURL(_ raw: String?) -> URL? {
        guard let raw, let url = URL(string: raw), url.scheme == "https" else { return nil }
        return url
    }
    var stremioURL: URL? {
        Self.detailURL(type: type, id: id)
    }
    static func detailURL(type: String, id: String) -> URL? {
        guard ["series", "movie"].contains(type), id.range(of: "^[A-Za-z0-9:_-]{1,160}$", options: .regularExpression) != nil else { return nil }
        return URL(string: "stremio:///detail/\(type)/\(id)" + (type == "movie" ? "/\(id)" : ""))
    }
}

nonisolated struct Catalogue: Decodable, Sendable {
    var series: [Title] = []
    var movies: [Title] = []
    var trendingSeries: [Title]?
    var trendingMovies: [Title]?
}
nonisolated struct Library: Decodable, Sendable {
    let connected: Bool
    let items: [Title]
}
nonisolated struct SearchResults: Decodable, Sendable { let items: [Title] }
nonisolated struct CollectionShelf: Decodable, Identifiable, Sendable {
    let id: String
    let title: String
    let type: String
    let items: [Title]
}
nonisolated struct ServiceCollections: Decodable, Sendable { let collections: [CollectionShelf] }
nonisolated struct PairResponse: Decodable, Sendable { let token: String }
nonisolated struct APIMessage: Decodable, Sendable { let error: String? }
nonisolated struct Acknowledgement: Decodable, Sendable { let ok: Bool }
nonisolated struct StremioPlayerLink: Decodable, Sendable {
    let uri: String
    let media: CueMedia?
    init(uri: String, media: CueMedia? = nil) { self.uri = uri; self.media = media }
    var validatedURL: URL? {
        guard uri.utf8.count <= 40000, let url = URL(string: uri), url.scheme == "stremio", (url.host ?? "").isEmpty,
              url.path.hasPrefix("/player/"), url.query == nil, url.fragment == nil else { return nil }
        return url
    }
}
nonisolated struct TVEvent: Decodable, Sendable {
    struct TitleRequest: Decodable, Sendable {
        let serial: Int64
        let type: String
        let id: String
        let service: String?
        var url: URL? {
            switch service ?? "stremio" {
            case "stremio": Title.detailURL(type: type, id: id)
            case "youtube": type == "video" ? Self.youtubeURL(id) : nil
            default: nil
            }
        }
        static func youtubeURL(_ id: String) -> URL? {
            guard id.range(of: "^[A-Za-z0-9_-]{11}$", options: .regularExpression) != nil else { return nil }
            return URL(string: "youtube://www.youtube.com/watch?v=\(id)")
        }
    }
    struct Platform: Decodable, Sendable { let home_request: Int64?; let open_request: TitleRequest? }
    let platform: Platform?
}
nonisolated struct TVConnection: Codable, Sendable {
    let address: String
    let token: String
}
nonisolated struct TVError: LocalizedError, Sendable {
    let message: String
    let status: Int
    var errorDescription: String? { message }
}

nonisolated enum TVAddress {
    static func normalise(_ text: String) -> URL? {
        let raw = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !raw.isEmpty, var parts = URLComponents(string: raw.contains("://") ? raw : "http://" + raw),
              ["http", "https"].contains(parts.scheme), let host = parts.host, parts.user == nil, parts.password == nil else { return nil }
        let octets = host.split(separator: ".").compactMap { Int($0) }
        let local = octets.count == 4 && octets.allSatisfy { (0...255).contains($0) } &&
            (octets[0] == 10 || (octets[0] == 192 && octets[1] == 168) || (octets[0] == 172 && (16...31).contains(octets[1])))
        guard local || host.hasSuffix(".local") || host == "127.0.0.1" || host == "localhost" else { return nil }
        if parts.port == nil { parts.port = 5400 }
        guard let port = parts.port, (1...65535).contains(port) else { return nil }
        parts.path = ""; parts.query = nil; parts.fragment = nil
        return parts.url
    }
}
