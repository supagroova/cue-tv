import SwiftUI

/// Keeps a failed intent available even if Core changed its in-memory membership.
struct LibraryMembershipControl: View {
    let member: Bool
    let save: @MainActor (Bool) async -> Bool
    private struct Request { let id = UUID(); let member: Bool }
    @State private var request: Request?
    @State private var failedMember: Bool?

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Button(request != nil ? "Saving library…" : failedMember != nil ? "Retry library save" : member ? "Remove from library" : "Add to library") {
                // Retain remote focus while saving; repeated presses do not enqueue writes.
                guard request == nil else { return }
                request = Request(member: failedMember ?? !member)
            }
            if failedMember != nil {
                Text("The library change could not be confirmed. Try again.").foregroundStyle(.orange)
            }
        }
        .task(id: request?.id) {
            guard let request else { return }
            let saved = await save(request.member)
            guard !Task.isCancelled else { return }
            failedMember = saved ? nil : request.member
            self.request = nil
        }
    }
}
