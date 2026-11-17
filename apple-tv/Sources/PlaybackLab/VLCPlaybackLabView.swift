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
                VLCFixtureSurface(url: url, audioIndex: requestedAudio, subtitleIndex: requestedSubtitle,
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

private struct VLCPlaybackSnapshot {
    let milliseconds: Int
    let audioTracks: Int
    let subtitleTracks: Int
    let selectedAudio: Int
    let selectedSubtitle: Int
    let state: VLCMediaPlayerState
}

private struct VLCFixtureSurface: UIViewRepresentable {
    let url: URL
    let audioIndex: Int
    let subtitleIndex: Int
    let paused: Bool
    let seekRequest: UUID?
    let seekMilliseconds: Int
    var progress: @MainActor (VLCPlaybackSnapshot) -> Void
    func makeCoordinator() -> Coordinator { Coordinator() }
    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.backgroundColor = .black
        context.coordinator.player.drawable = view
        do {
            let report = progress
            context.coordinator.player.media = VLCMedia(url: url)
            context.coordinator.player.play()
            let coordinator = context.coordinator
            coordinator.polling = Task { @MainActor in
                while !Task.isCancelled {
                    coordinator.applyTracks()
                    coordinator.applyControls()
                    let audio = coordinator.player.audioTracks
                    let subtitles = coordinator.player.textTracks
                    report(VLCPlaybackSnapshot(milliseconds: Int(coordinator.player.time.intValue),
                           audioTracks: audio.count, subtitleTracks: subtitles.count,
                           selectedAudio: (audio.firstIndex(where: { $0.isSelected }) ?? -1) + 1,
                           selectedSubtitle: (subtitles.firstIndex(where: { $0.isSelected }) ?? -1) + 1,
                           state: coordinator.player.state))
                    try? await Task.sleep(for: .milliseconds(100))
                }
            }
        }
        return view
    }
    func updateUIView(_ uiView: UIView, context: Context) {
        context.coordinator.audioIndex = audioIndex
        context.coordinator.subtitleIndex = subtitleIndex
        context.coordinator.paused = paused
        context.coordinator.seekRequest = seekRequest
        context.coordinator.seekMilliseconds = seekMilliseconds
    }
    static func dismantleUIView(_ uiView: UIView, coordinator: Coordinator) {
        coordinator.polling?.cancel()
        coordinator.polling = nil
        coordinator.player.stop()
        coordinator.player.drawable = nil
        coordinator.player.media = nil
    }
    final class Coordinator {
        let player = VLCMediaPlayer()
        var polling: Task<Void, Never>?
        var audioIndex = 0
        var subtitleIndex = 0
        var paused = false
        var seekRequest: UUID?
        var seekMilliseconds = 8000
        private var appliedPause = false
        private var appliedSeek: UUID?
        func applyControls() {
            guard player.state == .playing || player.state == .paused else { return }
            if paused != appliedPause {
                if paused { player.pause() } else { player.play() }
                appliedPause = paused
            }
            if let seekRequest, seekRequest != appliedSeek {
                player.time = VLCTime(int: Int32(seekMilliseconds))
                appliedSeek = seekRequest
            }
        }
        private var appliedAudio: Int?
        private var appliedSubtitle: Int?
        func applyTracks() {
            let audio = player.audioTracks
            let subtitles = player.textTracks
            if appliedAudio != audioIndex, audio.indices.contains(audioIndex) {
                audio[audioIndex].isSelectedExclusively = true
                appliedAudio = audioIndex
            }
            if appliedSubtitle != subtitleIndex {
                if subtitleIndex == -1 {
                    player.selectTextTracks([])
                    appliedSubtitle = subtitleIndex
                } else if subtitles.indices.contains(subtitleIndex) {
                    player.selectTextTracks([subtitles[subtitleIndex]])
                    appliedSubtitle = subtitleIndex
                }
            }
        }
    }
}
#endif
