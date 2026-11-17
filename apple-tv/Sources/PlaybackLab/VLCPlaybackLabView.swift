#if DEBUG
import SwiftUI
import VLCKit

/// Candidate decoder probe. Production selection still requires the full media matrix.
struct VLCPlaybackLabView: View {
    @State private var playing = false
    @State private var milliseconds = 0
    @FocusState private var stopFocused: Bool
    var body: some View {
        VStack {
            if playing {
                VLCFixtureSurface(milliseconds: $milliseconds).frame(height: 360)
                Text(milliseconds > 500 ? "VLC playback advanced" : "Waiting for VLC").accessibilityIdentifier("vlc-progress")
                Button("Stop VLC fixture") { playing = false; milliseconds = 0 }
                    .focused($stopFocused)
                    .task { stopFocused = true }
            } else {
                Button("Play MKV with VLC") { playing = true }
            }
        }
    }
}

private struct VLCFixtureSurface: UIViewRepresentable {
    @Binding var milliseconds: Int
    func makeCoordinator() -> Coordinator { Coordinator() }
    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.backgroundColor = .black
        context.coordinator.player.drawable = view
        if let url = Bundle.main.url(forResource: "episode01", withExtension: "mkv") {
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
