import Foundation

/// One playback owner, bounded text downloads, and files named independently of add-on data.
actor SubtitleFiles {
    enum Failure: Error { case invalidURL, response, tooLarge, unsupported, stopped, superseded, limit }
    private let root: URL
    private let session: URLSession
    private var active: Task<URL, any Error>?
    private var generation = UUID()
    private var closed = false
    private var cache: [String: URL] = [:]
    private static let byteLimit = 1024 * 1024

    init(root: URL = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)) throws {
        guard root.isFileURL, !FileManager.default.fileExists(atPath: root.path) else { throw Failure.invalidURL }
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false, attributes: [.posixPermissions: 0o700])
        self.root = root
        let configuration = URLSessionConfiguration.ephemeral
        configuration.httpCookieStorage = nil
        configuration.urlCredentialStorage = nil
        configuration.urlCache = nil
        configuration.timeoutIntervalForRequest = 15
        configuration.timeoutIntervalForResource = 20
        session = URLSession(configuration: configuration)
    }
    nonisolated static func validURL(_ value: String) -> URL? {
        guard value.utf8.count <= 2048, let url = URL(string: value),
              ["http", "https"].contains(url.scheme ?? ""), let host = url.host, !host.isEmpty,
              url.port != 0, url.user == nil, url.password == nil, url.fragment == nil else { return nil }
        return url
    }
    func load(_ value: String) async throws -> URL {
        guard !closed else { throw Failure.stopped }
        guard let url = Self.validURL(value) else { throw Failure.invalidURL }
        let token = UUID(); generation = token
        let previous = active
        previous?.cancel()
        _ = await previous?.result
        guard !closed, generation == token else { throw Failure.superseded }
        if let cached = cache[value] { return cached }
        guard cache.count < 32 else { throw Failure.limit }
        let session = session, root = root
        let task = Task { try await Self.fetch(url, session: session, root: root) }
        active = task
        do {
            let file = try await withTaskCancellationHandler { try await task.value } onCancel: { task.cancel() }
            guard !closed, generation == token, !Task.isCancelled else {
                try? FileManager.default.removeItem(at: file)
                throw Failure.superseded
            }
            active = nil
            cache[value] = file
            return file
        } catch {
            if generation == token { active = nil }
            throw error
        }
    }
    func stop() async throws {
        closed = true; generation = UUID()
        active?.cancel(); session.invalidateAndCancel()
        _ = await active?.result
        active = nil
        if FileManager.default.fileExists(atPath: root.path) { try FileManager.default.removeItem(at: root) }
        cache.removeAll()
    }
    private nonisolated static func fetch(_ url: URL, session: URLSession, root: URL) async throws -> URL {
        var request = URLRequest(url: url)
        request.setValue("text/vtt, application/x-subrip, text/plain", forHTTPHeaderField: "Accept")
        let (bytes, response) = try await session.bytes(for: request, delegate: SubtitleRedirects())
        defer { bytes.task.cancel() }
        return try await withTaskCancellationHandler {
            try await read(bytes, response: response, root: root)
        } onCancel: { bytes.task.cancel() }
    }
    private nonisolated static func read(_ bytes: URLSession.AsyncBytes, response: URLResponse, root: URL) async throws -> URL {
        guard let response = response as? HTTPURLResponse, response.statusCode == 200,
              response.url.flatMap({ validURL($0.absoluteString) }) != nil else { throw Failure.response }
        guard response.expectedContentLength <= byteLimit else { throw Failure.tooLarge }
        var data = Data()
        for try await byte in bytes {
            if data.count % 4096 == 0 { try Task.checkCancellation() }
            guard data.count < byteLimit else { throw Failure.tooLarge }
            data.append(byte)
        }
        try Task.checkCancellation()
        guard let text = String(data: data, encoding: .utf8), !text.contains("\0") else { throw Failure.unsupported }
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines.union(CharacterSet(charactersIn: "\u{FEFF}")))
        let isVTT = trimmed.hasPrefix("WEBVTT\n") || trimmed.hasPrefix("WEBVTT\r\n")
        let isSRT = trimmed.range(of: #"^\d+\r?\n\d{2}:\d{2}:\d{2},\d{3} --> "#, options: .regularExpression) != nil
        guard (isVTT || isSRT), trimmed.range(of: #"(?:\d{2}:)?\d{2}:\d{2}[.,]\d{3}\s+-->\s+(?:\d{2}:)?\d{2}:\d{2}[.,]\d{3}"#, options: .regularExpression) != nil else { throw Failure.unsupported }
        let file = root.appendingPathComponent(UUID().uuidString).appendingPathExtension(isVTT ? "vtt" : "srt")
        try data.write(to: file, options: .atomic)
        return file
    }
    deinit {
        active?.cancel(); session.invalidateAndCancel()
        let active = active, root = root
        Task {
            _ = await active?.result
            try? FileManager.default.removeItem(at: root)
        }
    }
}

nonisolated private final class SubtitleRedirects: NSObject, URLSessionTaskDelegate, Sendable {
    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse,
                    newRequest request: URLRequest, completionHandler: @escaping @Sendable (URLRequest?) -> Void) {
        let count = Int(task.taskDescription ?? "0") ?? 0
        guard count < 3, let url = request.url, SubtitleFiles.validURL(url.absoluteString) != nil,
              !(response.url?.scheme == "https" && url.scheme != "https") else { completionHandler(nil); return }
        task.taskDescription = String(count + 1)
        completionHandler(request)
    }
}
