#if DEBUG
import SwiftUI
import VLCKit

/// Candidate decoder probe. Production selection still requires the full media matrix.
struct VLCPlaybackLabView: View {
    @Environment(\.scenePhase) private var scenePhase
    @State private var url: URL?
    @State private var torrent = TorrentLabSession()
    @State private var milliseconds = 0
    @FocusState private var stopFocused: Bool
    var body: some View {
        VStack {
            if let url {
                VLCFixtureSurface(url: url, milliseconds: $milliseconds)
                    .id(url).frame(height: 360)
                Text(milliseconds > 500 ? "VLC playback advanced" : "Waiting for VLC").accessibilityIdentifier("vlc-progress")
                Button("Stop VLC fixture") { stop() }
                    .focused($stopFocused)
                    .task { stopFocused = true }
            } else {
                Button("Play MKV with VLC") {
                    stop()
                    url = Bundle.main.url(forResource: "episode01", withExtension: "mkv")
                }
                Button("Stream torrent with VLC") { stop(); torrent.start() }
                Text(torrent.status)
            }
        }
        .onChange(of: torrent.url) { _, value in
            guard let value else { return }
            url = value
        }
        .onChange(of: scenePhase) { _, phase in if phase != .active { stop() } }
        .onDisappear { stop() }
    }
    private func stop() {
        url = nil
        milliseconds = 0
        torrent.stop()
    }
}

private struct VLCFixtureSurface: UIViewRepresentable {
    let url: URL
    @Binding var milliseconds: Int
    func makeCoordinator() -> Coordinator { Coordinator() }
    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.backgroundColor = .black
        context.coordinator.player.drawable = view
        do {
            context.coordinator.player.media = VLCMedia(url: url)
            context.coordinator.player.play()
            let coordinator = context.coordinator
            let progress = $milliseconds
            coordinator.polling = Task { @MainActor in
                while !Task.isCancelled {
                    progress.wrappedValue = Int(coordinator.player.time.intValue)
                    try? await Task.sleep(for: .milliseconds(100))
                }
            }
        }
        return view
    }
    func updateUIView(_ uiView: UIView, context: Context) {}
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
    }
}
#endif
