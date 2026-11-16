import Foundation
import WebKit
import UIKit

@MainActor
final class RemoteModel: ObservableObject {
    @Published var target = TVTarget.initial()
    @Published var pageReady = false
    @Published var connecting = true
    @Published var waking = false
    @Published var errorMessage: String?
    @Published var loadRevision = 0
    @Published var showingYouTube = false
    @Published var managingYouTubeFeeds = false
    @Published private(set) var youtubeFeeds = YouTubeFeeds.restore(UserDefaults.standard.data(forKey: YouTubeFeeds.storageKey))
    weak var webView: WKWebView?
    weak var surface: RemoteSurfaceView?
    private let connection = TVConnection()
    private let pairing = PairingStore()
    private let youtube = YouTubePlaybackClient()
    private var checking = false
    init() { saveYouTubeFeeds() }
    private func saveYouTubeFeeds() {
        if let data = try? JSONEncoder().encode(youtubeFeeds) { UserDefaults.standard.set(data, forKey: YouTubeFeeds.storageKey) }
        surface?.useProfile(youtubeFeeds.selectedID)
        Task { await publishYouTubeFeeds() }
    }
    func youtubeFeedsJSON() -> [String: Any] {
        ["selected": youtubeFeeds.selectedID.uuidString, "profiles": youtubeFeeds.profiles.map { ["id": $0.id.uuidString, "name": $0.name] }]
    }
    func publishYouTubeFeeds() async {
        guard let data = try? JSONSerialization.data(withJSONObject: youtubeFeedsJSON()), let json = String(data: data, encoding: .utf8) else { return }
        _ = try? await webView?.evaluateJavaScript("window.dispatchEvent(new CustomEvent('tv:youtube-feeds', {detail:" + json + "}))")
    }
    func selectYouTubeFeed(_ id: UUID) { if youtubeFeeds.select(id) { saveYouTubeFeeds() } }
    func renameYouTubeFeed(_ id: UUID, to name: String) { if youtubeFeeds.rename(id, to: name) { saveYouTubeFeeds() } }
    @discardableResult func addYouTubeFeed(_ name: String) -> Bool {
        guard youtubeFeeds.add(name) != nil else { return false }; saveYouTubeFeeds(); return true
    }
    func removeYouTubeFeed(_ id: UUID) {
        guard youtubeFeeds.remove(id) else { return }; saveYouTubeFeeds()
        // Only this explicitly removed profile is cleared. The remote pairing uses another store.
        WKWebsiteDataStore(forIdentifier: id).removeData(ofTypes: WKWebsiteDataStore.allWebsiteDataTypes(), modifiedSince: .distantPast) {}
    }
    var previewLocal: Bool {
        #if DEBUG
        ProcessInfo.processInfo.environment["COUCHTV_REMOTE_PREVIEW"] == "1"
        #else
        false
        #endif
    }
    var pageURL: URL? { previewLocal ? URL(string: "http://127.0.0.1:5400/") : target.baseURL }
    func ownsPage(_ url: URL) -> Bool {
        guard let expected = pageURL else { return false }
        return url.scheme == expected.scheme && url.host == expected.host && url.port == expected.port
    }

    var previewOffline: Bool {
        #if DEBUG
        ProcessInfo.processInfo.arguments.contains("--preview-offline")
        #else
        false
        #endif
    }
    func connect() async {
        guard !checking else { return }; checking = true; defer { checking = false }
        // Returning from an account window must retain the current page and feed position.
        if pageReady { connecting = false; return }
        pairing.saveCurrentTarget(target)
        if previewOffline { connecting = false; return }
        if previewLocal { loadRevision += 1; return }
        connecting = true
        if await connection.isAvailable(target) { loadRevision += 1 }
        else { connecting = false }
    }
    func pageLoaded() { pageReady = true; connecting = false; errorMessage = nil }
    func pageFailed() { pageReady = false; connecting = false }
    func saveToken(_ token: String) { if !previewLocal { pairing.save(token, for: target) } }
    func openYouTubeFeed(_ video: YouTubeVideo) async throws {
        if previewLocal { throw TVRemoteError.pairingRequired }
        guard let token = pairing.token(for: target) else { throw TVRemoteError.pairingRequired }
        try await youtube.open(video, target: target, token: token)
        _ = try? await webView?.evaluateJavaScript("window.dispatchEvent(new Event('tv:youtube-feed-opened'))")
    }

    @discardableResult func wake() async throws -> Bool {
        guard !waking else { return false }
        waking = true; errorMessage = nil
        defer { waking = false }
        do {
            let result = try await connection.wake(target, token: pairing.token(for: target))
            if !pageReady { connecting = true; loadRevision += 1 }
            if case .readyToPair = result { errorMessage = TVRemoteError.pairingRequired.localizedDescription }
            UIImpactFeedbackGenerator(style: .light).impactOccurred()
            return true
        } catch {
            errorMessage = error.localizedDescription
            throw error
        }
    }
    func updateTarget(_ newTarget: TVTarget) {
        guard newTarget.isValid else { return }
        target = newTarget
        pairing.saveCurrentTarget(newTarget)
        if let data = try? JSONEncoder().encode(newTarget) { UserDefaults.standard.set(data, forKey: "tv.target") }
        pageReady = false; errorMessage = nil
        Task { await connect() }
    }
}
