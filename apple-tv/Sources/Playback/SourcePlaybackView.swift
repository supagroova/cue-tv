import SwiftUI
import VLCKit

struct SourcePlaybackView: View {
    private struct DiscoveryKey: Hashable { let generation: UUID?; let ready: Bool }
    let request: CorePlaybackRequest
    var discoverSubtitles: (@MainActor () async throws -> CoreSubtitleDiscovery)? = nil
    private var subtitleChoices: [CoreSubtitle] {
        var seen = Set<String>()
        return Array((request.subtitles + subtitles.discovered).filter { seen.insert($0.language + "\n" + $0.url).inserted }.prefix(32))
    }
    @Environment(\.dismiss) private var dismiss
    @Environment(\.scenePhase) private var scenePhase
    @State private var model = SourcePlaybackModel()
    @State private var controller = VLCPlaybackController()
    @State private var subtitles = PlaybackSubtitlesModel()
    @State private var paused = false
    @State private var audio = 0
    @State private var subtitle = -1
    @State private var audioCount = 0
    @State private var subtitleCount = 0
    @State private var milliseconds = 0
    @State private var seekRequest: UUID?
    @State private var seekTarget = 0
    @State private var hasPlayed = false
    @FocusState private var closeFocused: Bool

    var body: some View {
        VStack(spacing: 24) {
            if case .failed(let failure) = model.coordinator.state {
                Text(failure.message).accessibilityIdentifier("source-playback-error")
                Button("Retry source") { reset(); Task { guard await subtitles.stop() else { return }; subtitles = PlaybackSubtitlesModel(); await model.start(request) } }
            } else if let url = model.playbackURL {
                GeometryReader { geometry in
                VLCPlaybackSurface(controller: controller, url: url, audioIndex: audio, subtitleIndex: subtitle,
                    paused: paused, seekRequest: seekRequest, seekMilliseconds: seekTarget, externalSubtitleURL: subtitles.url, externalSubtitleID: subtitles.selectionID) { [token = model.generation] snapshot in
                    guard let token, token == model.generation else { return }
                    if let rejected = snapshot.failedSubtitleSelection { subtitles.decoderFailed(selectionID: rejected) }
                    milliseconds = snapshot.milliseconds
                    audioCount = snapshot.audioTracks; subtitleCount = snapshot.subtitleTracks
                    if snapshot.state == .error { controller.stop(); model.coordinator.receive(.failed(.decoder), generation: token) }
                    else if snapshot.state == .stopped, hasPlayed { stopAndClose() }
                    else if snapshot.state == .paused { model.coordinator.receive(.paused, generation: token) }
                    else if snapshot.state == .playing, snapshot.milliseconds > 0 {
                        if model.coordinator.state == .seeking, abs(snapshot.milliseconds - seekTarget) > 1000 { return }
                        hasPlayed = true
                        model.coordinator.receive(.playing, generation: token)
                        model.coordinator.reportProgress(milliseconds: snapshot.milliseconds, generation: token)
                    }
                }.id(url).frame(width: geometry.size.width, height: geometry.size.height).clipped()
                }.frame(maxHeight: .infinity)
                Text("\(milliseconds / 1000)s").monospacedDigit().accessibilityIdentifier("source-playback-time")
                HStack {
                    Button(paused ? "Resume" : "Pause") { paused.toggle() }
                    Button("Back 10 seconds") { seek(to: max(0, milliseconds - 10000)) }
                    Button("Forward 10 seconds") { seek(to: min(Int(Int32.max), milliseconds + 10000)) }
                    if audioCount > 1 { Button("Next audio track") { audio = (audio + 1) % audioCount } }
                    if subtitleCount > 0 { Button(subtitle == -1 && subtitles.url == nil ? "Enable subtitles" : "Next subtitle / Off") { subtitles.disable(); subtitle = subtitle + 1 >= subtitleCount ? -1 : subtitle + 1 } }
                }
            } else if model.coordinator.state == .choosingFile, let files = model.files {
                Text("Choose a video file").font(.title)
                ScrollView {
                    VStack {
                        ForEach(files.files ?? [], id: \.index) { file in
                            Button { Task { await model.select(fileIndex: file.index) } } label: {
                                Text(file.path).lineLimit(2)
                            }.disabled(file.padding || file.bytes == 0)
                        }
                        HStack {
                            if let offset = files.offset, offset > 0 { Button("Previous files") { Task { await model.showFiles(offset: max(0, offset - 24)) } } }
                            if let offset = files.offset, let total = files.total, offset + 24 < total { Button("More files") { Task { await model.showFiles(offset: offset + 24) } } }
                        }
                    }
                }
            } else { ProgressView("Preparing video…") }
            if !subtitleChoices.isEmpty, model.playbackURL != nil {
                ScrollView(.horizontal) {
                    HStack {
                        Button("Subtitles off") { subtitles.disable(); subtitle = -1 }
                        ForEach(Array(subtitleChoices.enumerated()), id: \.offset) { _, track in
                            Button(track.label ?? track.language) { Task { await subtitles.choose(track) } }
                        }
                    }.padding(.vertical, 12)
                }.frame(height: 90).focusSection()
            }
            if let error = subtitles.discoveryError { Text(error).foregroundStyle(.orange) }
            if subtitles.loading { Text("Loading subtitles…") }
            if let error = subtitles.error { Text(error).foregroundStyle(.orange) }
            HStack {
                Spacer()
                Button("Close") { stopAndClose() }.focused($closeFocused)
                Spacer()
            }.focusSection()
        }
        .padding(40)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color.black)
        .task {
            model.stopDecoder = { [controller] in controller.stop() }
            closeFocused = true; await model.start(request)
        }
        .task(id: DiscoveryKey(generation: model.generation, ready: model.playbackURL != nil)) {
            guard model.playbackURL != nil, let discoverSubtitles else { return }
            await subtitles.discover(using: discoverSubtitles)
        }
        .onChange(of: model.coordinator.state) { _, state in
            if case .failed = state { controller.stop() }
        }
        .onChange(of: scenePhase) { _, phase in if phase != .active { stopAndClose() } }
        .onDisappear { controller.stop(); Task { await model.stop(); await subtitles.stop() } }
    }
    private func reset() {
        controller.stop(); paused = false; audio = 0; subtitle = -1
        milliseconds = 0; hasPlayed = false; seekRequest = nil
    }
    private func seek(to target: Int) {
        guard let token = model.generation else { return }
        seekTarget = target; seekRequest = UUID()
        model.coordinator.receive(.seeking, generation: token)
    }
    private func stopAndClose() {
        controller.stop()
        Task { await model.stop(); if await subtitles.stop() { dismiss() } }
    }
}
