import Foundation

struct YouTubeFeedProfile: Codable, Identifiable, Equatable, Sendable {
    let id: UUID
    var name: String
}

struct YouTubeFeeds: Codable, Equatable, Sendable {
    private(set) var profiles: [YouTubeFeedProfile]
    private(set) var selectedID: UUID
    static let storageKey = "youtube.home.feeds.v1"
    static let maximum = 8
    static func restore(_ data: Data?) -> Self {
        if let data, let value = try? JSONDecoder().decode(Self.self, from: data),
           !value.profiles.isEmpty, value.profiles.count <= maximum,
           Set(value.profiles.map(\.id)).count == value.profiles.count,
           value.profiles.allSatisfy({ $0.id.uuidString != "00000000-0000-0000-0000-000000000000" }),
           value.profiles.allSatisfy({ clean($0.name) == $0.name }),
           value.profiles.contains(where: { $0.id == value.selectedID }) { return value }
        let first = YouTubeFeedProfile(id: UUID(), name: "My YouTube")
        return Self(profiles: [first], selectedID: first.id)
    }
    static func clean(_ name: String) -> String? {
        let value = name.components(separatedBy: .controlCharacters).joined(separator: " ")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return value.isEmpty || value.count > 40 ? nil : value
    }
    @discardableResult mutating func add(_ name: String) -> UUID? {
        guard profiles.count < Self.maximum, canUseName(name), let name = Self.clean(name) else { return nil }
        let profile = YouTubeFeedProfile(id: UUID(), name: name)
        profiles.append(profile); selectedID = profile.id; return profile.id
    }
    @discardableResult mutating func select(_ id: UUID) -> Bool {
        guard profiles.contains(where: { $0.id == id }) else { return false }
        selectedID = id; return true
    }
    @discardableResult mutating func rename(_ id: UUID, to name: String) -> Bool {
        guard let index = profiles.firstIndex(where: { $0.id == id }), canUseName(name, except: id), let name = Self.clean(name) else { return false }
        profiles[index].name = name; return true
    }
    func canUseName(_ name: String, except id: UUID? = nil) -> Bool {
        guard let name = Self.clean(name) else { return false }
        return !profiles.contains { $0.id != id && $0.name.caseInsensitiveCompare(name) == .orderedSame }
    }
    @discardableResult mutating func remove(_ id: UUID) -> Bool {
        guard profiles.count > 1, profiles.contains(where: { $0.id == id }) else { return false }
        profiles.removeAll { $0.id == id }
        if selectedID == id { selectedID = profiles[0].id }
        return true
    }
}
