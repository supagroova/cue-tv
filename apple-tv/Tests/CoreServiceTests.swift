import Foundation
import Testing
import Network
import Security
@testable import SittingRoomTV

@Suite struct CoreServiceTests {
@Test func sourceGroupsUseProviderIdentityWithoutRenumberingSelections() throws {
    let data = Data(#"{"status":"ready","loading":false,"video_id":"owned","offset":0,"total":4,"items":[{"index":0,"kind":"http","addon_index":4,"addon_name":"Twin"},{"index":1,"kind":"http","addon_index":5,"addon_name":"Twin"},{"index":2,"kind":"torrent","addon_index":4,"addon_name":"Twin"},{"index":3,"kind":"unsupported"}]}"#.utf8)
    let decoder = JSONDecoder(); decoder.keyDecodingStrategy = .convertFromSnakeCase
    let sources = try decoder.decode(CoreSnapshot.Sources.self, from: data)
    #expect(sources.groups.map(\.id) == [4, 5, nil])
    #expect(sources.groups.map(\.name) == ["Twin", "Twin", "Unknown add-on"])
    #expect(sources.groups.map { $0.items.map(\.index) } == [[0, 2], [1], [3]])
}

@Test func swiftCoreServiceRunsRealModelAndJoinsStop() async throws {
    let service = NativeCoreService()
    try await service.start()
    var state = try await service.snapshot()
    for _ in 0..<100 where state.status != .ready {
        try await Task.sleep(for: .milliseconds(10))
        state = try await service.snapshot()
    }
    #expect(state.status == .ready)
    #expect(try await service.accountIdentity() == nil)
    #expect(state.model?.interfaceLanguage == "eng")
    await #expect(throws: NativeCoreService.Failure.invalidRequest) { try await service.loadAddon("file:///private/manifest.json") }
    try await service.setInterfaceLanguage("fra")
    for _ in 0..<100 {
        state = try await service.snapshot()
        if state.model?.interfaceLanguage == "fra" && state.model?.pendingEffects == 0 { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.interfaceLanguage == "fra")
    #expect(state.model?.pendingEffects == 0)
    #expect(state.model?.error == nil)
    async let first: Void = service.stop()
    async let second: Void = service.stop()
    try await first
    try await second
    await #expect(throws: NativeCoreService.Failure.stopped) { try await service.setInterfaceLanguage("eng") }
    await #expect(throws: NativeCoreService.Failure.stopped) { _ = try await service.accountIdentity() }
    try await service.start()
    #expect(try await service.snapshot().session != state.session)
    try await service.stop()
}

@Test func swiftCoreLoadsManifestOverSimulatorHTTP() async throws {
    let fixture = try await CoreHTTPFixture.start()
    defer { fixture.listener.cancel() }
    let service = NativeCoreService()
    try await service.start()
    try await service.loadAddon("http://127.0.0.1:\(fixture.port)/private-config/manifest.json")
    var state = try await service.snapshot()
    for _ in 0..<200 {
        if state.model?.addon?.status == .ready { break }
        try await Task.sleep(for: .milliseconds(10))
        state = try await service.snapshot()
    }
    #expect(state.model?.addon?.status == .ready)
    #expect(state.model?.addon?.id == "org.cue.simulator")
    #expect(state.model?.addon?.name == "Simulator fixture")
    let originalCount = state.model?.addonCount
    try await service.installAddon(expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.addon?.installed == true && state.model?.pendingEffects == 0 { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.addon?.installed == true)
    #expect(state.model?.addonCount == originalCount.map { $0 + 1 })
    let beforePaging = try #require(state.model?.revision)
    try await service.showCatalogChoices(offset: 0, expectedRevision: beforePaging)
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.revision != beforePaging { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.catalogChoicesOffset == 0)
    let choice = try #require(state.model?.catalogs.first { $0.name == "Fixture catalogue" })
    try await service.loadCatalog(index: choice.index, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.catalog?.status == .ready { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.catalog?.items.first?.id == "cue-fixture")
    #expect(state.model?.catalog?.items.first?.name == "Simulator film")
    #expect(state.model?.catalog?.items.count == 24)
    #expect(choice.searchable)
    try await service.searchCatalog(index: choice.index, query: "needle", expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.catalog?.items.first?.name == "Search result" { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.catalog?.items.first?.name == "Search result")
    try await service.loadDetails(index: 0, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.details?.status == .ready { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.details?.id == "cue-fixture")
    #expect(state.model?.details?.name == "Detailed simulator film")
    #expect(state.model?.details?.description == "Owned title details")
    #expect(state.model?.details?.videoTotal == 30)
    #expect(state.model?.details?.videos.count == 24)
    #expect(state.model?.details?.seasons == [1, 2])
    let beforeLibrary = try #require(state.model?.revision)
    try await service.setDetailsLibrary(true, expectedRevision: beforeLibrary)
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.detailsInLibrary == true { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.detailsInLibrary == true)
    await #expect(throws: NativeCoreService.Failure.requestFailed("stale_details")) {
        try await service.setDetailsLibrary(false, expectedRevision: beforeLibrary)
    }
    try await service.setDetailsLibrary(false, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.detailsInLibrary == false { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.detailsInLibrary == false)

    let beforeWatched = try #require(state.model?.revision)
    try await service.setDetailsWatched(true, videoID: nil, expectedRevision: beforeWatched)
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.details?.watched == true { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.details?.watched == true)
    #expect(state.model?.detailsInLibrary == false)
    await #expect(throws: NativeCoreService.Failure.requestFailed("stale_details")) {
        try await service.setDetailsWatched(false, videoID: nil, expectedRevision: beforeWatched)
    }
    try await service.setDetailsWatched(false, videoID: nil, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.details?.watched == false { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.details?.watched == false)

    try await service.selectDetailsSeason(2, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.details?.selectedSeason == 2 { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.details?.selectedSeason == 2)
    #expect(state.model?.details?.videoTotal == 15)
    #expect(state.model?.details?.videos.first?.id == "cue-fixture:16")
    #expect(state.model?.details?.videos.last?.id == "cue-fixture:30")
    try await service.selectDetailsSeason(nil, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.details?.selectedSeason == nil { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.details?.videoTotal == 30)
    try await service.showDetailsVideos(offset: 24, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.details?.videoOffset == 24 { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.details?.videos.count == 6)
    #expect(state.model?.details?.videos.last?.id == "cue-fixture:30")
    try await service.loadSources(videoIndex: 5, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.sources?.status == .ready { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.sources?.videoId == "cue-fixture:30")
    #expect(state.model?.sources?.total == 31)
    #expect(state.model?.sources?.items.count == 24)
    #expect(state.model?.sources?.items.first?.kind == .torrent)
    #expect(state.model?.sources?.items.first?.sizeBytes == nil)
    try await service.showSources(offset: 24, expectedRevision: try #require(state.model?.revision))
    for _ in 0..<200 {
        state = try await service.snapshot()
        if state.model?.sources?.offset == 24 { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.sources?.items.count == 7)
    #expect(state.model?.sources?.items.last?.name == "Direct HTTPS")
    let handoff = try await service.prepareTorrent(index: 29, expectedRevision: try #require(state.model?.revision))
    #expect(handoff.magnet.contains("0123456789abcdef0123456789abcdef01234567"))
    #expect(handoff.metaId == "cue-fixture")
    #expect(handoff.videoId == "cue-fixture:30")
    #expect(handoff.kind == "movie")
    #expect(handoff.fileIndex == nil)
    #expect(handoff.fileMustInclude.isEmpty)
    #expect(handoff.playbackToken > 0)
    try await service.startPlayback(token: handoff.playbackToken)
    let seek = try await service.playback(token: handoff.playbackToken, action: .seek, time: 800, duration: 1000)
    #expect(seek?.timeWatched == 0)
    let played = try await service.playback(token: handoff.playbackToken, action: .played, time: 900, duration: 1000)
    #expect(played?.timeWatched == 100)
    #expect(played?.watched == false)
    try await service.checkpointPlayback(token: handoff.playbackToken)
    try await service.closePlayback(token: handoff.playbackToken)
    await #expect(throws: NativeCoreService.Failure.requestFailed("stale_playback")) {
        try await service.playback(token: handoff.playbackToken, action: .played, time: 950, duration: 1000)
    }
    await #expect(throws: NativeCoreService.Failure.invalidRequest) {
        try await service.playback(token: handoff.playbackToken, action: .seek, time: .max, duration: 1000)
    }
    let discovered = try await service.discoverSubtitles(metaId: handoff.metaId, videoId: handoff.videoId, kind: handoff.kind)
    #expect(discovered.tracks.map(\.id) == ["owned-resource"])
    #expect(discovered.failedProviders == 0)
    let stalled = Task { try await service.discoverSubtitles(metaId: handoff.metaId, videoId: handoff.videoId, kind: handoff.kind) }
    let reachedDeadline = ContinuousClock.now.advanced(by: .seconds(2))
    while await fixture.subtitles.requests < 2, ContinuousClock.now < reachedDeadline { try await Task.sleep(for: .milliseconds(5)) }
    try #require(await fixture.subtitles.requests == 2)
    // Playback must wait for an unadmitted ticket instead of failing a save just
    // because subtitle HTTP currently owns the private reply slot.
    let waitingPlayback = Task { try await service.playback(token: handoff.playbackToken, action: .pause) }
    let cancelledPlayback = Task { try await service.playback(token: handoff.playbackToken, action: .pause) }
    try await Task.sleep(for: .milliseconds(50))
    cancelledPlayback.cancel()
    await #expect(throws: CancellationError.self) { try await cancelledPlayback.value }
    let cancelStarted = ContinuousClock.now
    stalled.cancel()
    await #expect(throws: CancellationError.self) { try await stalled.value }
    // This token was closed above: after admission, preserve its native error.
    await #expect(throws: NativeCoreService.Failure.requestFailed("stale_playback")) { try await waitingPlayback.value }
    let closedDeadline = ContinuousClock.now.advanced(by: .seconds(2))
    while !(await fixture.subtitles.closed), ContinuousClock.now < closedDeadline { try await Task.sleep(for: .milliseconds(5)) }
    #expect(await fixture.subtitles.closed)
    #expect(cancelStarted.duration(to: .now) < .seconds(2))
    // A canceled HTTP query must release the serial C command owner promptly.
    try await service.setInterfaceLanguage("fra")
    var afterCancel = try await service.snapshot()
    for _ in 0..<100 where afterCancel.model?.interfaceLanguage != "fra" {
        try await Task.sleep(for: .milliseconds(10)); afterCancel = try await service.snapshot()
    }
    #expect(afterCancel.model?.interfaceLanguage == "fra")
    state = afterCancel

    await #expect(throws: NativeCoreService.Failure.requestFailed("stale_subtitle_identity")) {
        try await service.discoverSubtitles(metaId: handoff.metaId, videoId: "wrong-episode", kind: handoff.kind)
    }

    let https = try await service.prepareHTTPS(index: 30, expectedRevision: try #require(state.model?.revision))
    #expect(https.url == "https://media.example.test/video.mkv?signature=synthetic%2Bvalue")
    #expect(https.metaId == "cue-fixture")
    #expect(https.videoId == "cue-fixture:30")
    #expect(https.kind == "movie")
    await #expect(throws: NativeCoreService.Failure.requestFailed("stale_sources")) {
        try await service.prepareHTTPS(index: 30, expectedRevision: 0)
    }
    await #expect(throws: NativeCoreService.Failure.requestFailed("unsupported_source")) {
        try await service.prepareHTTPS(index: 29, expectedRevision: try #require(state.model?.revision))
    }
    await #expect(throws: NativeCoreService.Failure.requestFailed("stale_sources")) {
        try await service.prepareTorrent(index: 29, expectedRevision: 0)
    }
    for offset in [24, 27, 30] {
        try await service.showCatalogItems(offset: offset, expectedRevision: try #require(state.model?.revision))
        for _ in 0..<200 {
            state = try await service.snapshot()
            if state.model?.catalog?.offset == offset && state.model?.catalog?.status == .ready { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(state.model?.catalog?.items.count == (offset == 30 ? 0 : 3))
    }
    #expect(state.model?.catalog?.hasMore == false)
    #expect(state.error == nil)
    #expect(state.model?.error == nil)

    // A held network effect keeps the save receipt pending after mutation admission.
    try await service.loadAddon("http://127.0.0.1:\(fixture.port)/held/manifest.json")
    for _ in 0..<200 {
        state = try await service.snapshot()
        let requests = await fixture.heldManifest.requests
        if state.model?.addon?.status == .loading && requests == 1 { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    try #require(await fixture.heldManifest.requests == 1)
    try #require(state.model?.addon?.status == .loading)
    let libraryRevision = try #require(state.model?.revision)
    let saving = Task { try await service.setDetailsLibrary(true, expectedRevision: libraryRevision) }
    try await Task.sleep(for: .milliseconds(100))
    await #expect(throws: NativeCoreService.Failure.busy) {
        try await service.prepareTorrent(index: 29, expectedRevision: libraryRevision)
    }
    let libraryCancelStarted = ContinuousClock.now
    saving.cancel()
    await #expect(throws: CancellationError.self) { try await saving.value }
    try await service.setInterfaceLanguage("deu")
    for _ in 0..<100 {
        state = try await service.snapshot()
        if state.model?.interfaceLanguage == "deu" { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.interfaceLanguage == "deu")
    #expect(state.model?.detailsInLibrary == true, "Cancellation must not pretend to roll back an admitted save")
    #expect(libraryCancelStarted.duration(to: .now) < .seconds(2))
    let watchedRevision = try #require(state.model?.revision)
    let watchedSave = Task { try await service.setDetailsWatched(true, videoID: nil, expectedRevision: watchedRevision) }
    try await Task.sleep(for: .milliseconds(100))
    await #expect(throws: NativeCoreService.Failure.busy) {
        try await service.setDetailsLibrary(false, expectedRevision: watchedRevision)
    }
    let watchedCancelStarted = ContinuousClock.now
    watchedSave.cancel()
    await #expect(throws: CancellationError.self) { try await watchedSave.value }
    try await service.setInterfaceLanguage("eng")
    for _ in 0..<100 {
        state = try await service.snapshot()
        if state.model?.interfaceLanguage == "eng" { break }
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(state.model?.interfaceLanguage == "eng")
    #expect(state.model?.details?.watched == true)
    #expect(watchedCancelStarted.duration(to: .now) < .seconds(2))
    try await service.stop()
    for _ in 0..<100 where !(await fixture.heldManifest.closed) { try await Task.sleep(for: .milliseconds(10)) }
    #expect(await fixture.heldManifest.closed)
}

@Test func accountRestoreDoesNotFallBackToGuestWhenStorageIsEmpty() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: directory) }
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    let configuration = CorePersistenceConfiguration(directory: directory, key: Data(repeating: 49, count: 32))
    let account = NativeCoreService(persistence: configuration, startup: .restoreAccount("fixture-user"))
    try await account.start()
    var state = try await account.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10))
        state = try await account.snapshot()
    }
    #expect(state.status == .stopped)
    #expect(state.error == "core_start_failed")
    try await account.stop()
    // Failed account startup must release the store, so a guest can open it explicitly.
    let guest = NativeCoreService(persistence: configuration)
    try await guest.start()
    state = try await guest.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10))
        state = try await guest.snapshot()
    }
    #expect(state.status == .ready)
    #expect(state.model?.authenticated == false)
    try await guest.stop()
    let missingStore = NativeCoreService(startup: .restoreAccount("fixture-user"))
    await #expect(throws: NativeCoreService.Failure.invalidRequest) { try await missingStore.start() }
}

@Test func keychainBackedGuestSurvivesServiceRecreation() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
    let account = "test-" + UUID().uuidString
    let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
                               kSecAttrService as String: CoreStorage.keyService,
                               kSecAttrAccount as String: account]
    defer { SecItemDelete(query as CFDictionary); try? FileManager.default.removeItem(at: directory) }
    let store = CoreStorage(directory: directory, keyAccount: account)
    let first = NativeCoreService(persistence: try await store.prepare())
    try await first.start()
    try await first.setInterfaceLanguage("deu")
    var state = try await first.snapshot()
    for _ in 0..<200 {
        if state.model?.interfaceLanguage == "deu" && state.model?.pendingEffects == 0 { break }
        try await Task.sleep(for: .milliseconds(10)); state = try await first.snapshot()
    }
    #expect(state.model?.interfaceLanguage == "deu")
    #expect(state.model?.error == nil)
    try await first.stop()
    let second = NativeCoreService(persistence: try await store.prepare())
    try await second.start()
    state = try await second.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10)); state = try await second.snapshot()
    }
    #expect(state.model?.interfaceLanguage == "deu")
    try await second.stop()
    #expect(SecItemDelete(query as CFDictionary) == errSecSuccess)
    await #expect(throws: CoreStorage.Failure.missingKey) { try await store.prepare() }
    #expect(FileManager.default.fileExists(atPath: directory.appendingPathComponent("state.bin").path))
}

}


private actor SubtitleResourceProbe {
    var requests = 0
    var closed = false
    func arrived() -> Int { requests += 1; return requests }
    func disconnected() { closed = true }
}

nonisolated private final class CoreHTTPFixture: Sendable {
    let listener: NWListener
    let port: UInt16
    let subtitles: SubtitleResourceProbe
    let heldManifest: SubtitleResourceProbe
    init(listener: NWListener, port: UInt16, subtitles: SubtitleResourceProbe, heldManifest: SubtitleResourceProbe) {
        self.listener = listener; self.port = port; self.subtitles = subtitles; self.heldManifest = heldManifest
    }
    static func start() async throws -> CoreHTTPFixture {
        let listener = try NWListener(using: .tcp, on: .any)
        let subtitles = SubtitleResourceProbe()
        let heldManifest = SubtitleResourceProbe()
        listener.newConnectionHandler = { connection in
            connection.start(queue: .global())
            connection.receive(minimumIncompleteLength: 1, maximumLength: 4096) { request, _, _, _ in
                let requestText = request.map { String(decoding: $0, as: UTF8.self) } ?? ""
                if requestText.contains("/held/manifest.json") {
                    Task { _ = await heldManifest.arrived() }
                    connection.receive(minimumIncompleteLength: 1, maximumLength: 1) { _, _, complete, error in
                        if complete || error != nil { Task { await heldManifest.disconnected() } }
                        connection.cancel()
                    }
                    return
                }
                if requestText.contains("/subtitles/movie/cue-fixture") {
                    Task {
                        if await subtitles.arrived() == 1 {
                            let body = #"{"subtitles":[{"id":"owned-resource","lang":"eng","url":"https://fixture.test/captions.vtt"}]}"#
                            let response = "HTTP/1.1 200 OK\r\nContent-Length: \(body.utf8.count)\r\nConnection: close\r\n\r\n\(body)"
                            connection.send(content: Data(response.utf8), completion: .contentProcessed { _ in connection.cancel() })
                        } else {
                            connection.receive(minimumIncompleteLength: 1, maximumLength: 1) { _, _, complete, error in
                                if complete || error != nil { Task { await subtitles.disconnected() } }
                                connection.cancel()
                            }
                        }
                    }
                    return
                }
                let isMetadata = requestText.contains("/meta/movie/cue-fixture.json")
                let isCatalog = requestText.contains("/catalog/movie/fixture")
                let range = requestText.contains("skip=30") ? 30..<30 : requestText.contains("skip=27") ? 27..<30 : 0..<27
                let metas = range.map { index in ["id": index == 0 ? "cue-fixture" : "cue-fixture-\(index)", "name": index == 0 ? (requestText.contains("search=needle") ? "Search result" : "Simulator film") : "Film \(index)", "type": "movie"] }
                let catalogBody = String(decoding: try! JSONSerialization.data(withJSONObject: ["metas": metas]), as: UTF8.self)
                let videos: [[String: Any]] = (1...30).map { ["id": "cue-fixture:\($0)", "title": "Edition \($0)", "season": $0 <= 15 ? 1 : 2, "episode": ($0 - 1) % 15 + 1] }
                let metadataBody = String(decoding: try! JSONSerialization.data(withJSONObject: ["meta": ["id": "cue-fixture", "type": "movie", "name": "Detailed simulator film", "description": "Owned title details", "videos": videos]]), as: UTF8.self)
                let streams = (0..<30).map { ["infoHash": "0123456789abcdef0123456789abcdef01234567", "name": "Source \($0)"] } + [["url": "https://media.example.test/video.mkv?signature=synthetic%2Bvalue", "name": "Direct HTTPS"]]
                let sourceBody = String(decoding: try! JSONSerialization.data(withJSONObject: ["streams": streams]), as: UTF8.self)
                let body = requestText.contains("/stream/movie/") ? sourceBody : isCatalog
                    ? catalogBody
                    : isMetadata ? metadataBody
                    : #"{"id":"org.cue.simulator","name":"Simulator fixture","version":"1.0.0","description":"Owned fixture","resources":["catalog",{"name":"subtitles","types":["movie"],"idPrefixes":["cue-"]},{"name":"stream","types":["movie"],"idPrefixes":["cue-"]},{"name":"meta","types":["movie"],"idPrefixes":["cue-"]}],"types":["movie"],"catalogs":[{"id":"fixture","type":"movie","name":"Fixture catalogue","extra":[{"name":"skip"},{"name":"search"}]}]}"#
                let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: \(body.utf8.count)\r\nConnection: close\r\n\r\n\(body)"
                connection.send(content: Data(response.utf8), completion: .contentProcessed { _ in connection.cancel() })
            }
        }
        let port: UInt16 = try await withCheckedThrowingContinuation { continuation in
            listener.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    listener.stateUpdateHandler = nil
                    continuation.resume(returning: listener.port!.rawValue)
                case .failed(let error):
                    listener.stateUpdateHandler = nil
                    listener.cancel()
                    continuation.resume(throwing: error)
                default: break
                }
            }
            listener.start(queue: .global())
        }
        return CoreHTTPFixture(listener: listener, port: port, subtitles: subtitles, heldManifest: heldManifest)
    }
}

@Test func accountNamespacesUseIndependentKeysAndState() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let firstID = UUID(), secondID = UUID()
    defer {
        for id in [firstID, secondID] {
            let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
                kSecAttrService as String: CoreStorage.keyService,
                kSecAttrAccount as String: "account-v1:" + id.uuidString.lowercased()]
            SecItemDelete(query as CFDictionary)
        }
        try? FileManager.default.removeItem(at: root)
    }
    let firstStore = CoreStorage.account(id: firstID, root: root)
    let secondStore = CoreStorage.account(id: secondID, root: root)
    let firstConfig = try await firstStore.prepare()
    let secondConfig = try await secondStore.prepare()
    #expect(firstConfig.directory != secondConfig.directory)
    #expect(firstConfig.key != secondConfig.key)
    #expect(firstConfig.directory.lastPathComponent == firstID.uuidString.lowercased())
    let first = NativeCoreService(persistence: firstConfig)
    try await first.start()
    try await first.setInterfaceLanguage("fra")
    var state = try await first.snapshot()
    for _ in 0..<200 where state.model?.interfaceLanguage != "fra" || state.model?.pendingEffects != 0 {
        try await Task.sleep(for: .milliseconds(10)); state = try await first.snapshot()
    }
    #expect(state.model?.interfaceLanguage == "fra")
    try await first.stop()
    let second = NativeCoreService(persistence: secondConfig)
    try await second.start()
    state = try await second.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10)); state = try await second.snapshot()
    }
    #expect(state.model?.interfaceLanguage == "eng")
    try await second.stop()
    let restored = try await CoreStorage.account(id: firstID, root: root).prepare()
    #expect(restored.key == firstConfig.key)
    let reopened = NativeCoreService(persistence: restored)
    try await reopened.start()
    state = try await reopened.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10)); state = try await reopened.snapshot()
    }
    #expect(state.model?.interfaceLanguage == "fra")
    try await reopened.stop()
}

@Test func freshAccountServiceValidatesCredentialsAndCannotReuseStorage() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: directory) }
    let configuration = CorePersistenceConfiguration(directory: directory, key: Data(repeating: 83, count: 32))
    let service = NativeCoreService(persistence: configuration, startup: .newAccount)
    try await service.start()
    var state = try await service.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10)); state = try await service.snapshot()
    }
    #expect(state.status == .ready)
    await #expect(throws: NativeCoreService.Failure.invalidRequest) { try await service.login(email: "", password: "synthetic-password") }
    let revision = try #require(state.model?.revision)
    try await service.logout()
    for _ in 0..<200 where state.model?.revision == revision {
        try await Task.sleep(for: .milliseconds(10)); state = try await service.snapshot()
    }
    #expect(state.model?.revision != revision)
    #expect(state.model?.authenticated == false)
    try await service.stop()
    try await service.start()
    state = try await service.snapshot()
    for _ in 0..<200 where state.status == .starting {
        try await Task.sleep(for: .milliseconds(10)); state = try await service.snapshot()
    }
    #expect(state.status == .stopped)
    #expect(state.error == "core_start_failed")
    try await service.stop()
    let guest = NativeCoreService()
    try await guest.start()
    await #expect(throws: NativeCoreService.Failure.invalidRequest) { try await guest.login(email: "fixture@example.test", password: "synthetic-password") }
    try await guest.stop()
}

@Test func activeAccountSelectionRoundTripsAndIgnoresStaleSignOut() async throws {
    let label = "selection-test-" + UUID().uuidString
    let store = CoreAccountSelection(keyAccount: label)
    let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: CoreStorage.keyService, kSecAttrAccount as String: label]
    defer { SecItemDelete(query as CFDictionary) }
    #expect(try await store.load() == nil)
    let first = CoreAccountSelection.Account(namespace: UUID(), userID: "first-fixture-user")
    let second = CoreAccountSelection.Account(namespace: UUID(), userID: "second-fixture-user")
    try await store.save(first)
    var attributesQuery = query
    attributesQuery[kSecReturnAttributes as String] = true
    var attributesResult: CFTypeRef?
    #expect(SecItemCopyMatching(attributesQuery as CFDictionary, &attributesResult) == errSecSuccess)
    let attributes = try #require(attributesResult as? [String: Any])
    #expect(attributes[kSecAttrAccessible as String] as? String == kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly as String)
    #expect(attributes[kSecAttrSynchronizable as String] as? Bool != true)
    #expect(try await CoreAccountSelection(keyAccount: label).load() == first)
    await #expect(throws: CoreAccountSelection.Failure.alreadySelected) { try await store.selectNew(second) }
    #expect(try await store.load() == first)
    try await store.save(second)
    #expect(try await store.clear(ifMatching: first.namespace) == false)
    #expect(try await store.load() == second)
    #expect(try await store.clear(ifMatching: second.namespace) == true)
    #expect(try await store.load() == nil)
    try await store.selectNew(first)
    #expect(try await store.clear(ifMatching: first.namespace) == true)
    await #expect(throws: CoreAccountSelection.Failure.invalidRecord) {
        try await store.save(.init(namespace: UUID(), userID: ""))
    }
    try await store.save(first)
    let invalid = Data(#"{"version":99,"account":{"namespace":"00000000-0000-0000-0000-000000000000","userID":"fixture"}}"#.utf8)
    #expect(SecItemUpdate(query as CFDictionary, [kSecValueData as String: invalid] as CFDictionary) == errSecSuccess)
    await #expect(throws: CoreAccountSelection.Failure.invalidRecord) { try await store.load() }
}

@Test func discardingAccountStopsOwnerAndPreservesOtherAccount() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let firstID = UUID(), secondID = UUID()
    let firstStore = CoreStorage.account(id: firstID, root: root)
    let secondStore = CoreStorage.account(id: secondID, root: root)
    let first = try await firstStore.prepare(), second = try await secondStore.prepare()
    defer {
        for id in [firstID, secondID] {
            SecItemDelete([kSecClass as String:kSecClassGenericPassword, kSecAttrService as String:CoreStorage.keyService,
                kSecAttrAccount as String:"account-v1:" + id.uuidString.lowercased()] as CFDictionary)
        }
        try? FileManager.default.removeItem(at: root)
    }
    let service = NativeCoreService(persistence: first, startup: .newAccount)
    try await service.start()
    try await service.discardAccountStorage(firstStore)
    #expect(!FileManager.default.fileExists(atPath: first.directory.path))
    let keyQuery: [String: Any] = [kSecClass as String:kSecClassGenericPassword, kSecAttrService as String:CoreStorage.keyService,
        kSecAttrAccount as String:"account-v1:" + firstID.uuidString.lowercased()]
    #expect(SecItemCopyMatching(keyQuery as CFDictionary, nil) == errSecItemNotFound)
    #expect(try await secondStore.prepare().key == second.key)
    await #expect(throws: NativeCoreService.Failure.stopped) { try await service.start() }
    try await service.discardAccountStorage(firstStore)
    let guest = NativeCoreService(persistence: second)
    await #expect(throws: NativeCoreService.Failure.invalidRequest) { try await guest.discardAccountStorage(secondStore) }
    #expect(FileManager.default.fileExists(atPath: second.directory.path))
}

@Test func failedLoginAttemptRemovesCandidateAndDoesNotSelectAccount() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let selectionLabel = "login-selection-" + UUID().uuidString
    let selection = CoreAccountSelection(keyAccount: selectionLabel)
    let attempt = CoreAccountLogin(root: root, selection: selection)
    let namespace = attempt.namespace
    defer {
        for label in [selectionLabel, "account-v1:" + namespace.uuidString.lowercased()] {
            SecItemDelete([kSecClass as String:kSecClassGenericPassword, kSecAttrService as String:CoreStorage.keyService,
                kSecAttrAccount as String:label] as CFDictionary)
        }
        try? FileManager.default.removeItem(at: root)
    }
    await #expect(throws: NativeCoreService.Failure.invalidRequest) { _ = try await attempt.signIn(email: "", password: "synthetic-password") }
    #expect(try await selection.load() == nil)
    #expect(!FileManager.default.fileExists(atPath: root.appendingPathComponent(namespace.uuidString.lowercased()).path))
    let keyQuery: [String: Any] = [kSecClass as String:kSecClassGenericPassword, kSecAttrService as String:CoreStorage.keyService,
        kSecAttrAccount as String:"account-v1:" + namespace.uuidString.lowercased()]
    #expect(SecItemCopyMatching(keyQuery as CFDictionary, nil) == errSecItemNotFound)
    try await attempt.cancel()
}

@Test func savedAccountWithoutCiphertextFailsWithoutGuestFallback() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let label = "missing-selection-" + UUID().uuidString
    let selection = CoreAccountSelection(keyAccount: label)
    let account = CoreAccountSelection.Account(namespace: UUID(), userID: "fixture-user")
    defer {
        for key in [label, "account-v1:" + account.namespace.uuidString.lowercased()] {
            SecItemDelete([kSecClass as String:kSecClassGenericPassword, kSecAttrService as String:CoreStorage.keyService,
                kSecAttrAccount as String:key] as CFDictionary)
        }
        try? FileManager.default.removeItem(at: root)
    }
    try await selection.save(account)
    await #expect(throws: CoreStorage.Failure.missingState) { _ = try await NativeCoreService.selected(selection: selection, accountRoot: root) }
    #expect(try await selection.load() == account)
    #expect(!FileManager.default.fileExists(atPath: root.path))
}

@Test func cancelledLoginAttemptCannotStartLater() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let selection = CoreAccountSelection(keyAccount: "cancel-selection-" + UUID().uuidString)
    let attempt = CoreAccountLogin(root: root, selection: selection)
    try await attempt.cancel()
    await #expect(throws: CancellationError.self) { _ = try await attempt.signIn(email: "", password: "synthetic-password") }
    #expect(!FileManager.default.fileExists(atPath: root.path))
    #expect(try await selection.load() == nil)
}
