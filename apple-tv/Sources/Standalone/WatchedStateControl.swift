import SwiftUI

/// Retains an unconfirmed intent even when Core has already changed its memory state.
struct WatchedStateControl: View {
    let watched: Bool
    var subject: String? = nil
    let save: @MainActor (Bool) async -> Bool
    private struct Request { let id = UUID(); let watched: Bool }
    @State private var request: Request?
    @State private var failedIntent: Bool?

    private var label: String {
        if request != nil { return "Saving watched state…" }
        if failedIntent != nil { return "Retry watched save" }
        return watched ? "Mark as unwatched" : "Mark as watched"
    }
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Button(label) {
                guard request == nil else { return }
                request = Request(watched: failedIntent ?? !watched)
            }.accessibilityLabel(subject.map { "\(label): \($0)" } ?? label)
            if failedIntent != nil {
                Text("The watched change could not be confirmed. Try again.").foregroundStyle(.orange)
            }
        }
        .task(id: request?.id) {
            guard let request else { return }
            let saved = await save(request.watched)
            guard !Task.isCancelled else { return }
            failedIntent = saved ? nil : request.watched
            self.request = nil
        }
    }
}
