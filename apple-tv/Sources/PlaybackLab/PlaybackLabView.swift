#if DEBUG
import AVKit
import SwiftUI

struct PlaybackLabView: View {
    @Environment(\.scenePhase) private var scenePhase
    @State private var fixtures: [PlaybackFixture] = []
    @State private var player: AVPlayer?
    @State private var selected: String?
    @State private var error: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Playback Lab").font(.largeTitle)
            Text("Local decoder control · torrent engine not connected").foregroundStyle(.secondary)
            if let player {
                VideoPlayer(player: player).frame(height: 450)
                Button("Stop playback") { stop() }
            } else {
                ForEach(fixtures) { fixture in
                    Button("Play \(fixture.title) — \(fixture.durationSeconds)s") { start(fixture) }
                }
            }
            Text(selected.map { "Selected fixture: \($0)" } ?? "Select a generated fixture")
            if let error { Text(error).foregroundStyle(.orange) }
            Text("H.264 / AAC · 1080p SDR · generated test media").font(.caption)
        }
        .padding(60)
        .task {
            do { fixtures = try PlaybackFixture.load(from: .main) }
            catch { self.error = "The bundled fixture manifest could not be loaded." }
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
        player?.pause()
        player?.replaceCurrentItem(with: nil)
        player = nil
        selected = nil
    }
}
#endif
