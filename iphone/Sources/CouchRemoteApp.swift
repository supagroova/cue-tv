import SwiftUI

@main
struct CouchRemoteApp: App {
    var body: some Scene { WindowGroup { RemoteScreen().preferredColorScheme(.dark) } }
}

struct RemoteScreen: View {
    @StateObject private var model = RemoteModel()
    @State private var settings = false
    @Environment(\.scenePhase) private var scenePhase
    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            RemoteWebView(model: model).ignoresSafeArea().opacity(model.pageReady ? 1 : 0)
            if !model.pageReady {
                VStack(spacing: 28) {
                    HStack {
                        VStack(alignment: .leading, spacing: 5) {
                            Text(model.target.name).font(.headline)
                            Text(model.waking ? "Turning on…" : model.connecting ? "Connecting…" : "Ready when you are").font(.subheadline).foregroundStyle(.secondary)
                        }
                        Spacer()
                        Button { settings = true } label: { Image(systemName: "gearshape").frame(width: 44, height: 44) }.buttonStyle(.plain).accessibilityLabel("TV settings")
                    }
                    Spacer()
                    VStack(spacing: 24) {
                        Image(systemName: "tv").font(.system(size: 64, weight: .light)).foregroundStyle(.white.opacity(0.8)).accessibilityHidden(true)
                        Text(model.waking ? "Turning on your TV" : "Your TV, ready to watch.").font(.title2.weight(.semibold)).multilineTextAlignment(.center)
                        if model.connecting { ProgressView().controlSize(.large) }
                        else {
                            Button { Task { try? await model.wake() } } label: {
                                HStack(spacing: 10) { if model.waking { ProgressView().tint(.black) } else { Image(systemName: "power") }; Text(model.waking ? "Turning on…" : "Turn on TV") }
                                    .font(.headline).frame(maxWidth: .infinity).frame(minHeight: 56)
                            }.buttonStyle(.borderedProminent).tint(.white).foregroundStyle(.black).disabled(model.waking)
                        }
                        if let error = model.errorMessage { Text(error).font(.callout).foregroundStyle(.secondary).multilineTextAlignment(.center) }
                    }.frame(maxWidth: 360)
                    Spacer()
                    Button("Try connecting again") { Task { await model.connect() } }.font(.callout).disabled(model.connecting || model.waking)
                    Text("Connect your iPhone to your home Wi-Fi.").font(.footnote).foregroundStyle(.secondary)
                }.padding(28).frame(maxWidth: 520)
            }
        }
        .task { await model.connect() }
        .onChange(of: scenePhase) { _, phase in if phase == .active && !model.pageReady { Task { await model.connect() } } }
        .sheet(isPresented: $settings) { TargetSettings(target: model.target) { model.updateTarget($0) } }
        .sheet(isPresented: $model.managingYouTubeFeeds) { YouTubeFeedSettings(model: model) }
        .fullScreenCover(isPresented: $model.showingYouTube) { YouTubeHomeView() }
        #if DEBUG
        .onAppear {
            if ProcessInfo.processInfo.arguments.contains("--youtube-home") { model.showingYouTube = true }
        }
        #endif
    }
}

struct TargetSettings: View {
    @State var target: TVTarget
    let save: (TVTarget) -> Void
    @Environment(\.dismiss) private var dismiss
    var body: some View {
        NavigationStack {
            Form {
                Section("Your TV") {
                    TextField("TV name", text: $target.name)
                    TextField("IP address", text: $target.host).keyboardType(.numbersAndPunctuation).textInputAutocapitalization(.never).autocorrectionDisabled()
                    TextField("Network address", text: $target.macAddress).textInputAutocapitalization(.never).autocorrectionDisabled()
                }
                Section { Text("The network address is the TV’s Wi-Fi or Ethernet MAC address. It lets this iPhone wake the TV while its screen is off.").font(.footnote) }
            }
            .navigationTitle("TV Settings").navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } }
                ToolbarItem(placement: .confirmationAction) { Button("Save") { save(target); dismiss() }.disabled(!target.isValid) }
            }
        }
    }
}
