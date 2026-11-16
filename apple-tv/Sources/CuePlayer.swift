import SwiftUI
import AVFoundation
import MediaPlayer
@preconcurrency import VLCKit

nonisolated struct CueMedia: Decodable, Sendable {
    let playback: String
    let url: String
    let type: String
    let id: String
    let video: String
    let title: String
    let episodeTitle: String?
    let season: Int?
    let episode: Int?
    let resume: Int64
    var subtitle: String {
        [season.map { "Season \($0)" }, episode.map { "Episode \($0)" }, episodeTitle].compactMap { $0 }.joined(separator: " · ")
    }
    var validatedURL: URL? {
        guard playback.range(of: "^[a-f0-9]{32}$", options: .regularExpression) != nil,
              ["movie", "series"].contains(type), resume >= 0, resume <= 2_592_000_000,
              let target = URL(string: url), target.user == nil, target.password == nil, target.fragment == nil else { return nil }
        #if DEBUG
        if ProcessInfo.processInfo.environment["COUCHTV_PLAYER_PREVIEW"] == url,
           target.scheme == "http", target.host == "127.0.0.1" { return target }
        #endif
        guard target.scheme == "https", target.host == "torrentio.strem.fun", target.port == nil || target.port == 443,
              target.path.hasPrefix("/realdebrid/") || target.path.hasPrefix("/resolve/realdebrid/") else { return nil }
        return target
    }
}
nonisolated struct CueMediaReply: Decodable, Sendable { let media: CueMedia }

@MainActor enum CuePlayerPresenter {
    static func show(_ media: CueMedia, service: TVService) -> Bool {
        guard media.validatedURL != nil,
              let scene = UIApplication.shared.connectedScenes.compactMap({ $0 as? UIWindowScene }).first(where: { $0.activationState == .foregroundActive }),
              var presenter = scene.windows.first(where: \.isKeyWindow)?.rootViewController else { return false }
        while let next = presenter.presentedViewController { presenter = next }
        guard !(presenter is UIHostingController<CuePlayerScreen>) else { return false }
        let controller = UIHostingController(rootView: CuePlayerScreen(media: media, service: service))
        controller.modalPresentationStyle = .fullScreen; controller.overrideUserInterfaceStyle = .dark
        presenter.present(controller, animated: true)
        return true
    }
}

@MainActor @Observable final class CuePlayerModel: NSObject, VLCMediaPlayerDelegate {
    let media: CueMedia
    let player = VLCMediaPlayer(options: ["--quiet"])
    private let service: TVService
    var controls = true
    var playing = false
    var loading = true
    var failure: String?
    var position: Int64 = 0
    var duration: Int64 = 0
    private var monitor: Task<Void, Never>?
    private var stopped = false
    private var resumed = false
    private var didPlay = false
    private var lastSaved: Int64 = -1
    private var commandTokens: [(MPRemoteCommand, Any)] = []
    private var debugEvents: [String] = []
    init(media: CueMedia, service: TVService) { self.media = media; self.service = service; super.init(); player.delegate = self }
    nonisolated func mediaPlayerTimeChanged(_ notification: Notification) { Task { @MainActor [weak self] in if let self { position = max(0, Int64(player.time.intValue)) } } }
    nonisolated func mediaPlayerStateChanged(_ state: VLCMediaPlayerState) {
        let code = state.rawValue
        Task { @MainActor [weak self] in
            guard let self, !stopped else { return }; trace("state \(code)")
            if code == VLCMediaPlayerState.playing.rawValue { playing = true; didPlay = true; loading = false }
            if code == VLCMediaPlayerState.error.rawValue || code == VLCMediaPlayerState.stopped.rawValue && (duration == 0 || position < duration - 2000) {
                loading = false; controls = true; failure = "This source couldn’t play. Go back and choose another source."
            }
        }
    }
    private func trace(_ value: String) {
        #if DEBUG
        if ProcessInfo.processInfo.environment["COUCHTV_PREVIEW_URL"]?.contains("127.0.0.1") == true {
            debugEvents.append(value); if debugEvents.count > 80 { debugEvents.removeFirst() }
            UserDefaults.standard.set(debugEvents, forKey: "cue-player-validation-events")
        }
        #endif
    }
    func start(in surface: UIView) {
        guard monitor == nil, let url = media.validatedURL, let resource = VLCMedia(url: url) else { return }
        stopped = false
        trace("start visible \(surface.window != nil)")
        try? AVAudioSession.sharedInstance().setCategory(.playback, mode: .moviePlayback)
        try? AVAudioSession.sharedInstance().setActive(true)
        player.drawable = surface; player.media = resource; player.play()
        let remote = MPRemoteCommandCenter.shared()
        commandTokens = [(remote.playCommand, remote.playCommand.addTarget { [weak self] _ in Task { @MainActor in self?.player.play() }; return .success }),
                         (remote.pauseCommand, remote.pauseCommand.addTarget { [weak self] _ in Task { @MainActor in self?.suspend("remote pause") }; return .success }),
                         (remote.togglePlayPauseCommand, remote.togglePlayPauseCommand.addTarget { [weak self] _ in Task { @MainActor in self?.toggle() }; return .success })]
        monitor = Task { [weak self] in
            guard let self else { return }
            var ticks = 0
            var hidInitialControls = false
            var reportedStop = false
            while !Task.isCancelled {
                position = max(0, Int64(player.time.intValue)); duration = max(0, Int64(player.media?.length.intValue ?? 0))
                playing = player.isPlaying
                trace("tick \(ticks) state \(player.state.rawValue) time \(position) duration \(duration) fraction \(player.position) app \(UIApplication.shared.applicationState.rawValue)")
                if playing { didPlay = true; loading = false; reportedStop = false }
                if player.state == .error { failure = "This source couldn’t play. Go back and choose another source."; loading = false }
                if !resumed, player.isSeekable, didPlay { resumed = true; if media.resume > 0 { player.time = VLCTime(int: Int32(clamping: media.resume)) } }
                if playing && position >= 5000 && !hidInitialControls { hidInitialControls = true; controls = false }
                if didPlay && position >= 1000 && (lastSaved < 0 || position / 5000 != lastSaved / 5000) {
                    lastSaved = position; save(state: playing ? "playing" : "paused")
                }
                if didPlay && player.state == .stopped && !reportedStop { reportedStop = true; loading = false; controls = true; save(state: duration > 0 && position >= duration - 2000 ? "ended" : "stopped") }
                MPNowPlayingInfoCenter.default().nowPlayingInfo = [MPMediaItemPropertyTitle: media.title, MPMediaItemPropertyAlbumTitle: media.subtitle,
                    MPNowPlayingInfoPropertyElapsedPlaybackTime: Double(position) / 1000, MPMediaItemPropertyPlaybackDuration: Double(duration) / 1000,
                    MPNowPlayingInfoPropertyPlaybackRate: playing ? 1.0 : 0.0]
                ticks += 1
                do { try await Task.sleep(for: .seconds(1)) } catch { break }
            }
        }
    }
    func toggle() { trace("toggle \(player.state.rawValue)"); if player.isPlaying { player.pause(); controls = true } else { player.play() }; save(state: player.isPlaying ? "playing" : "paused") }
    func suspend(_ reason: String) { trace(reason); player.pause(); controls = true }
    func seek(_ seconds: Int64) { guard player.isSeekable else { return }; player.time = VLCTime(int: Int32(clamping: max(0, min(duration > 0 ? duration : Int64.max, position + seconds * 1000)))) }
    func stop() {
        guard !stopped else { return }; stopped = true
        trace("stop")
        save(state: "stopped"); monitor?.cancel(); monitor = nil
        player.stop(); player.drawable = nil
        for (command, token) in commandTokens { command.removeTarget(token) }; commandTokens = []
        MPNowPlayingInfoCenter.default().nowPlayingInfo = nil
        try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
    }
    private func save(state: String) {
        guard didPlay else { return }
        let offset = position, length = duration, media = media, service = service
        Task { try? await service.progress(media, position: offset, duration: length, state: state) }
    }
    func audio(_ index: Int) { player.selectTrack(at: index, type: .audio) }
    func subtitles(_ index: Int?) { if let index { player.selectTrack(at: index, type: .text) } else { player.deselectAllTextTracks() } }
    static func clock(_ value: Int64) -> String { let seconds = max(0, value / 1000); return String(format: "%d:%02d", seconds / 60, seconds % 60) }
}

private final class VisibleVideoSurface: UIView {
    var ready: ((UIView) -> Void)?
    override func layoutSubviews() { super.layoutSubviews(); if window != nil && bounds.width > 0 && bounds.height > 0 { ready?(self) } }
}
private struct CueVideoSurface: UIViewRepresentable {
    let model: CuePlayerModel
    func makeUIView(context: Context) -> UIView { let view = VisibleVideoSurface(); view.backgroundColor = .black; view.ready = { [weak model] in model?.start(in: $0) }; return view }
    func updateUIView(_ uiView: UIView, context: Context) {}
}

struct CuePlayerScreen: View {
    @State private var model: CuePlayerModel
    @Environment(\.dismiss) private var dismiss
    init(media: CueMedia, service: TVService) { _model = State(initialValue: CuePlayerModel(media: media, service: service)) }
    var body: some View {
        ZStack {
            CueVideoSurface(model: model).ignoresSafeArea()
            if model.controls || model.failure != nil {
                LinearGradient(colors: [.black.opacity(0.6), .clear, .black.opacity(0.92)], startPoint: .top, endPoint: .bottom)
                    .ignoresSafeArea().allowsHitTesting(false)
            }
            if model.loading { ProgressView("Opening your source…").padding(32).background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 18)) }
            VStack(alignment: .leading, spacing: 24) {
                if model.controls || model.failure != nil {
                    HStack { Text("Cue TV").font(.headline); Spacer(); Button("Back to browsing", systemImage: "chevron.left") { close() } }
                    Spacer()
                    Text(model.media.title).font(.title.bold())
                    if !model.media.subtitle.isEmpty { Text(model.media.subtitle).foregroundStyle(.secondary) }
                    if let failure = model.failure { Text(failure).foregroundStyle(.orange) }
                    ProgressView(value: Double(model.position), total: Double(max(1, model.duration))).tint(.white)
                    HStack {
                        Text(CuePlayerModel.clock(model.position)).monospacedDigit(); Spacer()
                        Text(CuePlayerModel.clock(model.duration)).monospacedDigit()
                    }.font(.callout)
                    HStack(spacing: 28) {
                        Button("Back 10 seconds", systemImage: "gobackward.10") { model.seek(-10) }.labelStyle(.iconOnly)
                        Button(model.playing ? "Pause" : "Play", systemImage: model.playing ? "pause.fill" : "play.fill") { model.toggle() }.labelStyle(.iconOnly)
                        Button("Forward 10 seconds", systemImage: "goforward.10") { model.seek(10) }.labelStyle(.iconOnly)
                        Spacer()
                        Menu("Audio", systemImage: "speaker.wave.2") {
                            ForEach(Array(model.player.audioTracks.enumerated()), id: \.offset) { index, track in Button(track.trackName.isEmpty ? "Audio track \(index + 1)" : track.trackName) { model.audio(index) } }
                        }
                        Menu("Subtitles", systemImage: "captions.bubble") {
                            Button("Off") { model.subtitles(nil) }
                            ForEach(Array(model.player.textTracks.enumerated()), id: \.offset) { index, track in Button(track.trackName.isEmpty ? "Subtitle track \(index + 1)" : track.trackName) { model.subtitles(index) } }
                        }
                    }
                } else {
                    Spacer()
                    Button("Show playback controls") { model.controls = true }.opacity(0.02).accessibilityLabel("Show playback controls")
                }
            }.padding(70)
        }
        .preferredColorScheme(.dark).tint(.white)
        .onPlayPauseCommand { model.toggle() }
        .onExitCommand { close() }
        .onMoveCommand { direction in if !model.controls { if direction == .left { model.seek(-10) } else if direction == .right { model.seek(10) } else { model.controls = true } } }
        .onReceive(NotificationCenter.default.publisher(for: UIApplication.willResignActiveNotification)) { _ in model.suspend("app inactive") }
        .onDisappear { model.stop() }
    }
    private func close() { model.stop(); dismiss() }
}
