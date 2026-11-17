import Foundation

/// Owns disposable media only. File operations run on this actor, not the UI actor.
actor TorrentCacheStore {
    static let shared = TorrentCacheStore(root: FileManager.default.temporaryDirectory
        .appendingPathComponent("cue-torrent-cache-v1", isDirectory: true))
    enum Failure: Error { case busy, unexpectedEntry, unsafeRoot, invalidLease }
    private let root: URL
    private var prepared = false
    private var active: URL?

    init(root: URL) { self.root = root.standardizedFileURL }

    func acquire() throws -> URL {
        guard active == nil else { throw Failure.busy }
        if !prepared {
            if (try? root.resourceValues(forKeys: [.isSymbolicLinkKey]).isSymbolicLink) == true {
                throw Failure.unsafeRoot
            }
            try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
            let entries = try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil)
            // Fail closed on unrecognized content; never delete arbitrary files.
            guard entries.allSatisfy({ UUID(uuidString: $0.lastPathComponent) != nil }) else {
                throw Failure.unexpectedEntry
            }
            for entry in entries { try FileManager.default.removeItem(at: entry) }
            prepared = true
        }
        let directory = root.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
        active = directory
        return directory
    }

    /// Invoke only before native start or after its shutdown ticket confirms storage closure.
    func release(_ directory: URL) throws {
        guard active == directory else { throw Failure.invalidLease }
        if FileManager.default.fileExists(atPath: directory.path) {
            try FileManager.default.removeItem(at: directory)
        }
        active = nil
    }
}
