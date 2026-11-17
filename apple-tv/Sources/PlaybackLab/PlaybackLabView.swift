#if DEBUG
import AVKit
import SwiftUI

struct PlaybackLabView: View {
    @Environment(\.scenePhase) private var scenePhase
    @State private var fixtures: [PlaybackFixture] = []
    @State private var player: AVPlayer?
    @State private var selected: String?
    @State private var advanced = false
    @State private var playbackSeconds = 0.0
    @State private var firstFrameEvidence: String?
    @State private var error: String?
    @State private var torrent = TorrentLabSession()

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            if ProcessInfo.processInfo.environment["CUE_VLC_LAB"] == "1" {
                VLCPlaybackLabView()
            } else {
                Text("Playback Lab").font(.largeTitle)
                Text("Decoder controls and controlled-peer torrent proof").foregroundStyle(.secondary)
                if let player {
                    VideoPlayer(player: player).frame(height: 450)
                    Text(String(format: "%.1f", playbackSeconds)).font(.caption).accessibilityIdentifier("playback-seconds")
                    Text(advanced ? "Playback advanced" : "Waiting for video").accessibilityIdentifier("playback-progress")
                    if let firstFrameEvidence { Text(firstFrameEvidence).accessibilityIdentifier("first-frame-evidence") }
                    Button("Stop playback") { stop() }
                } else {
                    ForEach(fixtures) { fixture in
                        Button("Play \(fixture.title) — \(fixture.durationSeconds)s") { start(fixture) }
                    }
                }
                if player == nil {
                    Button("Stream torrent fixture") { stop(); torrent.start() }
                    Text(torrent.status)
                }
                Text(selected.map { "Selected fixture: \($0)" } ?? "Select a generated fixture")
                if let error { Text(error).foregroundStyle(.orange) }
                Text("H.264 / AAC · 1080p SDR · generated test media").font(.caption)
            }
        }
        .padding(60)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(Color.black.ignoresSafeArea())
        .tint(.blue)
        .task {
            do { fixtures = try PlaybackFixture.load(from: .main) }
            catch { self.error = "The bundled fixture manifest could not be loaded." }
        }
        .task(id: selected) {
            while !Task.isCancelled, let player {
                playbackSeconds = player.currentTime().seconds
                if !advanced, player.currentTime().seconds > 0.5 {
                    advanced = true
                    if torrent.totalBytes > 0 {
                        firstFrameEvidence = "First playback progress: \(torrent.verifiedBytes)/\(torrent.totalBytes) verified bytes"
                    }
                }
                try? await Task.sleep(for: .milliseconds(100))
            }
        }
        .onChange(of: torrent.url) { _, url in
            guard let url else { return }
            selected = "Direct torrent · fixture peer"
            let playback = AVPlayer(url: url)
            player = playback
            playback.play()
        }
        .onDisappear { stop() }
        .onChange(of: scenePhase) { _, phase in if phase != .active { stop() } }
    }

    private func start(_ fixture: PlaybackFixture) {
        stop()
        guard let url = fixture.url(in: .main) else {
            error = "The selected fixture is missing."
            return
        }
        error = nil
        selected = fixture.id
        let playback = AVPlayer(url: url)
        player = playback
        playback.play()
    }

    private func stop() {
        torrent.stop()
        player?.pause()
        player?.replaceCurrentItem(with: nil)
        player = nil
        selected = nil
        advanced = false
        playbackSeconds = 0
        firstFrameEvidence = nil
    }
}
#endif
