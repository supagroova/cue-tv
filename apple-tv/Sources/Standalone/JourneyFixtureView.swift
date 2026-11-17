#if DEBUG
import SwiftUI

/// Installs an owned HTTP add-on through the real Core; never substitutes a UI snapshot.
struct JourneyFixtureView: View {
    @State private var model: StandaloneModel?
    @State private var failed = false
    @State private var cleared = false
    var body: some View {
        Group {
            if cleared { Text("Journey storage cleared") }
            else if let model { StandaloneHome(model: model) }
            else if failed { Text("Journey fixture could not start") }
            else { ProgressView("Installing owned fixture…") }
        }
        .task {
            guard model == nil, !cleared else { return }
            var owner: NativeCoreService?
            do {
                let environment = ProcessInfo.processInfo.environment
                let service: NativeCoreService
                if let rawID = environment["CUE_JOURNEY_STORAGE"] {
                    guard let id = UUID(uuidString: rawID) else { throw CoreStorage.Failure.unsafeDirectory }
                    let root = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
                        .appendingPathComponent("CueJourneyTests", isDirectory: true)
                    let storage = CoreStorage.account(id: id, root: root)
                    if environment["CUE_JOURNEY_CLEAR_STORAGE"] == "1" {
                        try await storage.removeUnopenedAccount()
                        cleared = true
                        return
                    }
                    service = NativeCoreService(persistence: try await storage.prepare())
                } else { service = NativeCoreService() }
                owner = service
                try await service.start()
                try await service.loadAddon("http://127.0.0.1:42427/manifest.json")
                var installed = false
                for _ in 0..<100 {
                    try Task.checkCancellation()
                    let state = try await service.snapshot()
                    if let state = state.model, state.addon?.status == .ready {
                        if state.addon?.installed == true {
                            model = StandaloneModel(service: service)
                            return
                        }
                        if !installed {
                            try await service.installAddon(expectedRevision: state.revision)
                            installed = true
                        }
                    }
                    try await Task.sleep(for: .milliseconds(100))
                }
                throw NativeCoreService.Failure.requestTimeout
            } catch {
                try? await owner?.stop()
                failed = true
            }
        }
    }
}
#endif
