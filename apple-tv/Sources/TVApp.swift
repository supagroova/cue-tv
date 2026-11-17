import SwiftUI

@main struct SittingRoomApp: App {
    #if DEBUG
    @State private var showingHTTPSFixture = ProcessInfo.processInfo.environment["CUE_HTTPS_TEST_FIXTURE"] == "1"
    @State private var showingPlaybackLab = ProcessInfo.processInfo.environment["CUE_PLAYBACK_LAB"] == "1"
    #endif
    @Environment(\.scenePhase) private var scenePhase
    var body: some Scene {
        WindowGroup {
            Group {
                #if DEBUG
                if ProcessInfo.processInfo.environment["CUE_JOURNEY_FIXTURE"] == "1" { JourneyFixtureView() }
                else { StandaloneHome() }
                #else
                StandaloneHome()
                #endif
            }
            #if DEBUG
            .overlay(alignment: .topTrailing) {
                Button("Playback Lab") { showingPlaybackLab = true }
                    .padding(30)
                    .accessibilityIdentifier("playback-lab-open")
            }
            .fullScreenCover(isPresented: $showingPlaybackLab) { PlaybackLabView() }
            .fullScreenCover(isPresented: $showingHTTPSFixture) {
                let port = ProcessInfo.processInfo.environment["CUE_HTTPS_TEST_UNTRUSTED"] == "1" ? 42426 : 42425
                SourcePlaybackView(request: .https(CoreHTTPSRequest(metaId: "owned-fixture", videoId: "owned-fixture", kind: "movie", url: "https://127.0.0.1:\(port)/episode01.mp4")))
            }
            #endif
            .preferredColorScheme(.dark)

        }
    }
}

private struct ConnectView: View {
    @Bindable var model: WatchModel
    var body: some View {
        VStack(alignment: .leading, spacing: 26) {
            Image(systemName: "tv").font(.system(size: 64, weight: .light)).padding(.bottom, 8)
            Text("Your TV. Together.").font(.system(size: 58, weight: .bold))
            Text("Connect to Sitting Room TV to bring your Stremio library and iPhone remote to Apple TV.")
                .font(.title3).foregroundStyle(.secondary)
            TextField("TV address", text: $model.address).textContentType(.URL).autocorrectionDisabled()
            SecureField("Six-digit connection code", text: $model.code).keyboardType(.numberPad)
            if let error = model.error { Text(error).foregroundStyle(.orange).font(.body) }
            Button { Task { await model.connect() } } label: {
                if model.pairing { ProgressView().frame(width: 230) } else { Text("Connect").frame(width: 230) }
            }.disabled(model.pairing || model.code.count != 6)
            Text("In TV on your iPhone, open More Controls, then Connect Apple TV. Enter the code here.")
                .font(.callout).foregroundStyle(.secondary)
        }
        .frame(maxWidth: 850, alignment: .leading)
        .padding(80)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(LinearGradient(colors: [Color(white: 0.12), .black], startPoint: .topLeading, endPoint: .bottomTrailing))
    }
}
