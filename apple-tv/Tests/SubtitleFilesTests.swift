import Foundation
import Network
import Testing
@testable import SittingRoomTV

@Suite struct SubtitleFilesTests {
    @MainActor @Test func turningOffCancelsPendingSelectionAndCloseRemovesSelectedFile() async throws {
        let fixture = try await SubtitleHTTPFixture.start()
        defer { fixture.listener.cancel() }
        let model = PlaybackSubtitlesModel()
        let loading = Task { await model.choose(CoreSubtitle(id: "slow", language: "eng", label: nil, url: fixture.url("stall"))) }
        try await Task.sleep(for: .milliseconds(100))
        model.disable()
        await loading.value
        #expect(model.url == nil)
        #expect(!model.loading)
        await model.choose(CoreSubtitle(id: "good", language: "eng", label: nil, url: fixture.url("good")))
        let file = try #require(model.url)
        #expect(FileManager.default.fileExists(atPath: file.path))
        let previousSelection = try #require(model.selectionID)
        await model.choose(CoreSubtitle(id: "good", language: "eng", label: nil, url: fixture.url("good")))
        model.decoderFailed(selectionID: previousSelection)
        #expect(model.url == file)
        #expect(model.error == nil)
        #expect(await model.stop())
        #expect(!FileManager.default.fileExists(atPath: file.deletingLastPathComponent().path))
        await model.choose(CoreSubtitle(id: "late", language: "eng", label: nil, url: fixture.url("good")))
        #expect(model.url == nil)
    }

    @Test func downloadsOwnedTextAndDeletesItAfterStop() async throws {
        let fixture = try await SubtitleHTTPFixture.start()
        defer { fixture.listener.cancel() }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let files = try SubtitleFiles(root: root)
        let url = try await files.load(fixture.url("good"))
        #expect(url.pathExtension == "vtt")
        #expect(try String(contentsOf: url, encoding: .utf8) == "WEBVTT\n\n00:00:00.000 --> 00:00:10.000\nOwned captions\n")
        #expect(try await files.load(fixture.url("good")) == url)
        try await files.stop()
        #expect(!FileManager.default.fileExists(atPath: root.path))
        await #expect(throws: (any Error).self) { try await files.load(fixture.url("good")) }
    }
    @Test func refusesUnsafeURLsRedirectsAndUnboundedOrNonSubtitleBodies() async throws {
        let fixture = try await SubtitleHTTPFixture.start()
        defer { fixture.listener.cancel() }
        for input in ["file:///private/a.vtt", "https://user:secret@example.test/a.vtt", "https://example.test/a#fragment",
                      fixture.url("declared-large"), fixture.url("streamed-large"), fixture.url("html"), fixture.url("unsafe-redirect")] {
            let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
            let files = try SubtitleFiles(root: root)
            await #expect(throws: (any Error).self) { try await files.load(input) }
            #expect(try FileManager.default.contentsOfDirectory(atPath: root.path).isEmpty)
            try await files.stop()
        }
    }
    @Test func callerCancellationJoinsStalledTransferWithoutLeavingAFile() async throws {
        let fixture = try await SubtitleHTTPFixture.start()
        defer { fixture.listener.cancel() }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let files = try SubtitleFiles(root: root)
        let download = Task { try await files.load(fixture.url("stall")) }
        try await Task.sleep(for: .milliseconds(100))
        let start = ContinuousClock.now
        download.cancel()
        await #expect(throws: (any Error).self) { try await download.value }
        #expect(start.duration(to: .now) < .seconds(2))
        #expect(try FileManager.default.contentsOfDirectory(atPath: root.path).isEmpty)
        try await files.stop()
    }
    @Test func stopCancelsStalledTransferAndPreventsLateFileCreation() async throws {
        let fixture = try await SubtitleHTTPFixture.start()
        defer { fixture.listener.cancel() }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let files = try SubtitleFiles(root: root)
        let download = Task { try await files.load(fixture.url("stall")) }
        try await Task.sleep(for: .milliseconds(100))
        try await files.stop()
        await #expect(throws: (any Error).self) { try await download.value }
        #expect(!FileManager.default.fileExists(atPath: root.path))
    }
}

nonisolated private final class SubtitleHTTPFixture: Sendable {
    let listener: NWListener
    let port: UInt16
    init(listener: NWListener, port: UInt16) { self.listener = listener; self.port = port }
    func url(_ path: String) -> String { "http://127.0.0.1:\(port)/\(path)" }
    static func start() async throws -> SubtitleHTTPFixture {
        let listener = try NWListener(using: .tcp, on: .any)
        listener.newConnectionHandler = { connection in
            connection.start(queue: .global())
            connection.receive(minimumIncompleteLength: 1, maximumLength: 4096) { data, _, _, _ in
                let request = String(decoding: data ?? Data(), as: UTF8.self)
                var body = Data("WEBVTT\n\n00:00:00.000 --> 00:00:10.000\nOwned captions\n".utf8)
                var header = "HTTP/1.1 200 OK\r\nConnection: close\r\n"
                let stall = request.contains("/stall")
                if request.contains("/unsafe-redirect") { header = "HTTP/1.1 302 Found\r\nLocation: file:///private/a.vtt\r\nContent-Length: 0\r\n"; body = Data() }
                else if request.contains("/declared-large") { header += "Content-Length: 1048577\r\n"; body = Data() }
                else if request.contains("/streamed-large") { body = Data(repeating: 65, count: 1048577) }
                else if request.contains("/html") { body = Data("<html>Not a subtitle</html>".utf8) }
                else if stall { header += "Content-Length: 1000\r\n"; body = Data() }
                connection.send(content: Data((header + "\r\n").utf8) + body, completion: .contentProcessed { _ in if !stall { connection.cancel() } })
            }
        }
        let port: UInt16 = try await withCheckedThrowingContinuation { continuation in
            listener.stateUpdateHandler = { state in
                switch state {
                case .ready: listener.stateUpdateHandler = nil; continuation.resume(returning: listener.port!.rawValue)
                case .failed(let error): listener.stateUpdateHandler = nil; listener.cancel(); continuation.resume(throwing: error)
                default: break
                }
            }
            listener.start(queue: .global())
        }
        return SubtitleHTTPFixture(listener: listener, port: port)
    }
}
