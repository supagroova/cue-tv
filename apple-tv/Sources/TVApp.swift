import SwiftUI

@main struct SittingRoomApp: App {
    @State private var model = WatchModel()
    #if DEBUG
    @State private var showingPlaybackLab = ProcessInfo.processInfo.environment["CUE_PLAYBACK_LAB"] == "1"
    #endif
    @Environment(\.scenePhase) private var scenePhase
    var body: some Scene {
        WindowGroup {
            Group {
                if model.connected { WatchHome(model: model) }
                else { ConnectView(model: model) }
            }
            #if DEBUG
            .overlay(alignment: .topTrailing) {
                Button("Playback Lab") { showingPlaybackLab = true }
                    .padding(30)
                    .accessibilityIdentifier("playback-lab-open")
            }
            .fullScreenCover(isPresented: $showingPlaybackLab) { PlaybackLabView() }
            #endif
            .preferredColorScheme(.dark)
            .tint(.white)
            .task { await model.refresh() }
            .task(id: scenePhase == .active && model.connected) {
                if scenePhase == .active && model.connected { await model.observeRemote() }
            }
            .onChange(of: scenePhase) { _, phase in
                if phase == .active { Task { await model.refresh() } }
            }
            .onOpenURL { url in Task { await model.accept(url) } }
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
