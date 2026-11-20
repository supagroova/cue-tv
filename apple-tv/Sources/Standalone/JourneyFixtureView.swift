#if DEBUG
import SwiftUI

/// Installs an owned HTTP add-on through the real Core; never substitutes a UI snapshot.
struct JourneyFixtureView: View {
    @State private var model: StandaloneModel?
    @State private var failed = false
    var body: some View {
        Group {
            if let model { StandaloneHome(model: model) }
            else if failed { Text("Journey fixture could not start") }
            else { ProgressView("Installing owned fixture…") }
        }
        .task {
            guard model == nil else { return }
            let service = NativeCoreService()
            do {
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
                try? await service.stop()
                failed = true
            }
        }
    }
}
#endif
