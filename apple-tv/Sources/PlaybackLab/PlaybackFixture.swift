import Foundation

struct PlaybackFixture: Decodable, Identifiable, Sendable {
    let id: String
    let title: String
    let filename: String
    let sha256: String
    let durationSeconds: Int

    private enum CodingKeys: String, CodingKey { case id, title, filename, sha256, durationSeconds }

    init(from decoder: any Decoder) throws {
        let fields = try decoder.container(keyedBy: CodingKeys.self)
        id = try fields.decode(String.self, forKey: .id)
        title = try fields.decode(String.self, forKey: .title)
        filename = try fields.decode(String.self, forKey: .filename)
        sha256 = try fields.decode(String.self, forKey: .sha256)
        durationSeconds = try fields.decode(Int.self, forKey: .durationSeconds)
        guard !id.isEmpty, !title.isEmpty,
              filename.range(of: #"^[a-zA-Z0-9_-]+\.mp4$"#, options: .regularExpression) != nil,
              sha256.range(of: #"^[a-f0-9]{64}$"#, options: .regularExpression) != nil,
              durationSeconds > 0 else {
            throw DecodingError.dataCorrupted(.init(codingPath: decoder.codingPath, debugDescription: "Invalid bundled playback fixture"))
        }
    }

    func url(in bundle: Bundle) -> URL? {
        bundle.url(forResource: filename, withExtension: nil)
    }

    static func load(from bundle: Bundle) throws -> [PlaybackFixture] {
        guard let url = bundle.url(forResource: "playback-fixtures", withExtension: "json") else {
            throw CocoaError(.fileNoSuchFile)
        }
        return try JSONDecoder().decode([PlaybackFixture].self, from: Data(contentsOf: url))
    }
}
