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
    @State private var lifecycle = PlaybackSessionCoordinator()
    @State private var token: UUID?
    @FocusState private var stopFocused: Bool
    var body: some View {
        VStack {
            if let url {
                VLCFixtureSurface(url: url, audioIndex: requestedAudio, subtitleIndex: requestedSubtitle) { [generation = token] value, audio, subtitles, selectedA, selectedS in
                    guard let generation, generation == token else { return }
                    milliseconds = value
                    audioTracks = audio; subtitleTracks = subtitles
                    selectedAudio = selectedA; selectedSubtitle = selectedS
                    if value > 500 { lifecycle.receive(.playing, generation: generation) }
                }
                    .id(url).frame(height: 360)
                Text(milliseconds > 500 ? "VLC playback advanced" : "Waiting for VLC").accessibilityIdentifier("vlc-progress")
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
                    Button("Next subtitle track") { requestedSubtitle = (requestedSubtitle + 1) % subtitleTracks }
                }
            } else {
                Button("Play MKV with VLC") {
                    start(torrentSource: false)
                }
                Button("Stream torrent with VLC") { start(torrentSource: true) }
                Text(torrent.status)
                if case .failed(let failure) = lifecycle.state {
                    Text("Playback failed: \(String(describing: failure))")
                }
            }
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
                torrent.start()
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

private struct VLCFixtureSurface: UIViewRepresentable {
    let url: URL
    let audioIndex: Int
    let subtitleIndex: Int
    var progress: @MainActor (Int, Int, Int, Int, Int) -> Void
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
                    let audio = coordinator.player.audioTracks
                    let subtitles = coordinator.player.textTracks
                    report(Int(coordinator.player.time.intValue), audio.count, subtitles.count,
                           (audio.firstIndex(where: { $0.isSelected }) ?? -1) + 1,
                           (subtitles.firstIndex(where: { $0.isSelected }) ?? -1) + 1)
                    try? await Task.sleep(for: .milliseconds(100))
                }
            }
        }
        return view
    }
    func updateUIView(_ uiView: UIView, context: Context) {
        context.coordinator.audioIndex = audioIndex
        context.coordinator.subtitleIndex = subtitleIndex
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
        private var appliedAudio: Int?
        private var appliedSubtitle: Int?
        func applyTracks() {
            let audio = player.audioTracks
            let subtitles = player.textTracks
            if appliedAudio != audioIndex, audio.indices.contains(audioIndex) {
                audio[audioIndex].isSelectedExclusively = true
                appliedAudio = audioIndex
            }
            if appliedSubtitle != subtitleIndex, subtitles.indices.contains(subtitleIndex) {
                player.selectTextTracks([subtitles[subtitleIndex]])
                appliedSubtitle = subtitleIndex
            }
        }
    }
}
#endif
