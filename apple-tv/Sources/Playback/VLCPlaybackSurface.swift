import SwiftUI
import VLCKit

struct VLCPlaybackSnapshot {
    let milliseconds: Int
    let audioTracks: Int
    let subtitleTracks: Int
    let selectedAudio: Int
    let selectedSubtitle: Int
    let state: VLCMediaPlayerState
    var failedSubtitleSelection: UUID? = nil
    var durationMilliseconds: Int = 0
    var reposition: PlaybackReposition? = nil
    var bufferingProgress: Float? = nil
}

@MainActor final class VLCPlaybackController {
    fileprivate weak var coordinator: VLCPlaybackSurface.Coordinator?
    func stop() { coordinator?.stop() }
}

@MainActor private final class VLCVideoContainer: UIView {
    override func layoutSubviews() {
        super.layoutSubviews()
        // VLC attaches its rendering view after playback begins. Keep that view
        // inside the drawable when asynchronous controls change available space.
        for child in subviews { child.frame = bounds }
    }
    override func didAddSubview(_ subview: UIView) {
        super.didAddSubview(subview)
        subview.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        subview.frame = bounds
    }
}

struct VLCPlaybackSurface: UIViewRepresentable {
    var controller: VLCPlaybackController? = nil
    let url: URL
    let audioIndex: Int
    let subtitleIndex: Int
    let paused: Bool
    let seekRequest: UUID?
    let seekMilliseconds: Int
    var externalSubtitleURL: URL? = nil
    var externalSubtitleID: UUID? = nil
    var progress: @MainActor (VLCPlaybackSnapshot) -> Void
    func makeCoordinator() -> Coordinator { Coordinator() }
    func makeUIView(context: Context) -> UIView {
        let view = VLCVideoContainer()
        view.backgroundColor = .black
        view.accessibilityIdentifier = "vlc-video-surface"
        view.isAccessibilityElement = true
        view.accessibilityLabel = "Video"
        controller?.coordinator = context.coordinator
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
                           state: coordinator.player.state, failedSubtitleSelection: coordinator.subtitleLoadFailed ? coordinator.appliedSubtitleID : nil,
                           durationMilliseconds: max(0, Int(coordinator.player.media?.length.intValue ?? 0)),
                           reposition: coordinator.reposition, bufferingProgress: coordinator.bufferingProgress))
                    try? await Task.sleep(for: .milliseconds(100))
                }
            }
        }
        return view
    }
    func updateUIView(_ uiView: UIView, context: Context) {
        context.coordinator.externalSubtitleURL = externalSubtitleURL
        context.coordinator.externalSubtitleID = externalSubtitleID
        context.coordinator.audioIndex = audioIndex
        context.coordinator.subtitleIndex = subtitleIndex
        context.coordinator.paused = paused
        context.coordinator.seekRequest = seekRequest
        context.coordinator.seekMilliseconds = seekMilliseconds
    }
    static func dismantleUIView(_ uiView: UIView, coordinator: Coordinator) {
        coordinator.stop()
    }
    final class Coordinator: NSObject, VLCMediaPlayerDelegate {
        let player = PlaybackLogging.makePlayer()
        private(set) var bufferingProgress: Float?
        private var acceptingEvents = true
        override init() {
            super.init()
            player.delegate = self
        }
        nonisolated func mediaPlayerBufferingChanged(_ progress: Float) {
            Task { @MainActor [weak self] in
                guard let self, self.acceptingEvents else { return }
                self.bufferingProgress = progress.isFinite && (0...1).contains(progress) ? progress : nil
            }
        }
        func stop() {
            acceptingEvents = false
            bufferingProgress = nil
            player.delegate = nil
            polling?.cancel(); polling = nil
            player.stop(); player.drawable = nil; player.media = nil
        }
        var polling: Task<Void, Never>?
        var audioIndex = 0
        var subtitleIndex = 0
        var paused = false
        var seekRequest: UUID?
        var seekMilliseconds = 8000
        private(set) var reposition: PlaybackReposition?
        private var appliedPause = false
        private var appliedSeek: UUID?
        func applyControls() {
            guard player.state == .playing || player.state == .paused else { return }
            if paused != appliedPause {
                if paused { player.pause() } else { player.play() }
                appliedPause = paused
            }
            if let seekRequest, seekRequest != appliedSeek {
                let target = min(max(0, seekMilliseconds), Int(Int32.max))
                player.time = VLCTime(int: Int32(target))
                reposition = PlaybackReposition(id: seekRequest, target: target)
                appliedSeek = seekRequest
            }
        }
        var externalSubtitleURL: URL?
        var externalSubtitleID: UUID?
        fileprivate var appliedSubtitleID: UUID?
        fileprivate var appliedExternalSubtitleURL: URL?
        private var loadedSubtitles: [URL: String] = [:]
        private var attemptedSubtitles: Set<URL> = []
        private var pendingSubtitle: (url: URL, selectionID: UUID, previous: Set<String>, deadline: ContinuousClock.Instant)?
        var subtitleLoadFailed = false
        private var appliedAudio: Int?
        private var appliedSubtitle: Int?
        func applyTracks() {
            let audio = player.audioTracks
            let subtitles = player.textTracks
            if appliedAudio != audioIndex, audio.indices.contains(audioIndex) {
                audio[audioIndex].isSelectedExclusively = true
                appliedAudio = audioIndex
            }
            if let pending = pendingSubtitle {
                if let track = subtitles.first(where: { !pending.previous.contains($0.trackId) && $0.isSelected }) {
                    loadedSubtitles[pending.url] = track.trackId
                    pendingSubtitle = nil; appliedSubtitle = nil
                } else if ContinuousClock.now >= pending.deadline {
                    pendingSubtitle = nil; subtitleLoadFailed = true
                    appliedExternalSubtitleURL = pending.url; appliedSubtitleID = pending.selectionID
                } else { return }
            }
            if let url = externalSubtitleURL, let selectionID = externalSubtitleID {
                if appliedExternalSubtitleURL != url || appliedSubtitleID != selectionID {
                    guard player.state == .playing || player.state == .paused else { return }
                    if let id = loadedSubtitles[url], let track = subtitles.first(where: { $0.trackId == id }) {
                        let needsRefresh = !track.isSelected
                        let position = max(0, player.time.intValue)
                        player.selectTextTracks([track])
                        // VLC has already demuxed a long cue when a loaded track is toggled off.
                        // Refresh that cue at the current position without adding another slave.
                        if needsRefresh, player.isSeekable {
                            player.time = VLCTime(int: position)
                            reposition = PlaybackReposition(id: UUID(), target: Int(position))
                        }
                        appliedExternalSubtitleURL = url; appliedSubtitleID = selectionID
                        subtitleLoadFailed = false
                    } else if attemptedSubtitles.insert(url).inserted {
                        let previous = Set(subtitles.map(\.trackId))
                        subtitleLoadFailed = player.addPlaybackSlave(url, type: .subtitle, enforce: true) != 0
                        if subtitleLoadFailed { appliedExternalSubtitleURL = url; appliedSubtitleID = selectionID }
                        else { pendingSubtitle = (url, selectionID, previous, .now.advanced(by: .seconds(5))) }
                    } else { subtitleLoadFailed = true; appliedExternalSubtitleURL = url; appliedSubtitleID = selectionID }
                }
                return
            } else if appliedExternalSubtitleURL != nil {
                appliedExternalSubtitleURL = nil; appliedSubtitleID = nil
                subtitleLoadFailed = false
                appliedSubtitle = nil
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
