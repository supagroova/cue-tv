import SwiftUI
import VLCKit

struct VLCPlaybackSnapshot {
    let milliseconds: Int
    let audioTracks: Int
    let subtitleTracks: Int
    let selectedAudio: Int
    let selectedSubtitle: Int
    let state: VLCMediaPlayerState
}

struct VLCPlaybackSurface: UIViewRepresentable {
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
