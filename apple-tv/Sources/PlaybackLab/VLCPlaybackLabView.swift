#if DEBUG
import SwiftUI
import VLCKit

/// Candidate decoder probe. Production selection still requires the full media matrix.
struct VLCPlaybackLabView: View {
    @Environment(\.scenePhase) private var scenePhase
    @State private var url: URL?
    @State private var torrent = TorrentLabSession()
    @State private var milliseconds = 0
    @State private var audioTracks = 0
    @State private var subtitleTracks = 0
    @State private var requestedAudio = 0
    @State private var requestedSubtitle = 0
    @State private var selectedAudio = 0
    @State private var selectedSubtitle = 0
    @State private var requestedPause = false
    @State private var paused = false
    @State private var seekRequest: UUID?
    @State private var seeking = false
    @State private var firstProgress: String?
    @State private var hasPlayed = false
    private var seekMilliseconds: Int { ProcessInfo.processInfo.environment["CUE_VLC_LONG_SEEK"] == "1" ? 600000 : 8000 }
    @State private var lifecycle = PlaybackSessionCoordinator()
    @State private var token: UUID?
    @FocusState private var stopFocused: Bool
    var body: some View {
        VStack {
            if let url {
                VLCPlaybackSurface(url: url, audioIndex: requestedAudio, subtitleIndex: requestedSubtitle,
                                  paused: requestedPause, seekRequest: seekRequest, seekMilliseconds: seekMilliseconds) { [generation = token] snapshot in
                    guard let generation, generation == token else { return }
                    if snapshot.state == .stopped, hasPlayed { stop(); return }
                    milliseconds = snapshot.milliseconds
                    if milliseconds > 500 { hasPlayed = true }
                    if firstProgress == nil, milliseconds > 500, torrent.totalBytes > 0 {
                        firstProgress = "VLC first progress: \(torrent.verifiedBytes)/\(torrent.totalBytes)"
                    }
                    audioTracks = snapshot.audioTracks; subtitleTracks = snapshot.subtitleTracks
                    selectedAudio = snapshot.selectedAudio; selectedSubtitle = snapshot.selectedSubtitle
                    paused = snapshot.state == .paused
                    if snapshot.state == .error { lifecycle.receive(.failed(.decoder), generation: generation); return }
                    if seeking, snapshot.milliseconds < seekMilliseconds - 500 { return }
                    seeking = false
                    if paused { lifecycle.receive(.paused, generation: generation) }
                    else if snapshot.milliseconds > 500 {
                        lifecycle.receive(.playing, generation: generation)
                        lifecycle.reportProgress(milliseconds: snapshot.milliseconds, generation: generation)
                    }
                }
                    .id(url).frame(height: 360)
                if let file = torrent.selectedFile { Text("Selected media file: \(file)") }
                Text(paused ? "VLC paused" : "VLC active")
                Text(milliseconds > 500 ? "VLC playback advanced" : "Waiting for VLC").accessibilityIdentifier("vlc-progress")
                if let firstProgress { Text(firstProgress).accessibilityIdentifier("vlc-first-progress") }
                Text(verbatim: String(milliseconds)).accessibilityIdentifier("vlc-time")
                Text("Audio tracks: \(audioTracks); subtitle tracks: \(subtitleTracks)")
                Text("Selected audio: \(selectedAudio); subtitle: \(selectedSubtitle)")
                Button("Stop VLC fixture") { stop() }
                    .focused($stopFocused)
                    .task { stopFocused = true }
                if audioTracks > 1 {
                    Button("Next audio track") { requestedAudio = (requestedAudio + 1) % audioTracks }
                }
                if subtitleTracks > 0 {
                    Button("Next subtitle track") { requestedSubtitle = requestedSubtitle + 1 >= subtitleTracks ? -1 : requestedSubtitle + 1 }
                }
                Button(requestedPause ? "Resume" : "Pause") { requestedPause.toggle() }
                Button("Seek to \(seekMilliseconds / 1000) seconds") {
                    if let token { lifecycle.receive(.seeking, generation: token) }
                    seeking = true; seekRequest = UUID()
                }
            } else {
                Button("Play MKV with VLC") {
                    start(torrentSource: false)
                }
                Button("Stream torrent with VLC") { start(torrentSource: true) }
                Text(torrent.status)
                if case .failed(let failure) = lifecycle.state {
                    Text(failure.message).accessibilityIdentifier("playback-failure")
                }
            }
        }
        .onChange(of: torrent.failure) { _, failure in
            guard let failure, let token else { return }
            lifecycle.receive(.failed(failure), generation: token)
        }
        .onChange(of: torrent.url) { _, value in
            guard let value else { return }
            guard let token else { return }
            lifecycle.receive(.buffering, generation: token)
            url = value
        }
        .onChange(of: scenePhase) { _, phase in if phase != .active { stop() } }
        .onDisappear { stop() }
    }
    private func start(torrentSource: Bool) {
        url = nil; milliseconds = 0; token = nil
        requestedAudio = 0; requestedSubtitle = 0
        requestedPause = false; paused = false; seekRequest = nil; seeking = false
        firstProgress = nil; hasPlayed = false
        Task {
            let next = await lifecycle.begin {
                url = nil; milliseconds = 0; token = nil
                torrent.stop()
                while torrent.pendingCleanups > 0 {
                    try? await Task.sleep(for: .milliseconds(50))
                }
            }
            guard let next else { return }
            token = next
            if torrentSource {
                lifecycle.receive(.discovering, generation: next)
                await torrent.start()
            } else {
                let fixture = ProcessInfo.processInfo.environment["CUE_VLC_MATRIX"] == "1" ? "hevc-tracks" : "episode01"
                url = Bundle.main.url(forResource: fixture, withExtension: "mkv")
                lifecycle.receive(url == nil ? .failed(.invalidMetadata) : .buffering, generation: next)
            }
        }
    }
    private func stop() {
        url = nil
        milliseconds = 0
        token = nil
        Task { await lifecycle.stop() }
    }
}

#endif
