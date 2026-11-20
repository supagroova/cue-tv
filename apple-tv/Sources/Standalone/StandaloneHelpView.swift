import SwiftUI

struct StandaloneHelpView: View {
    let onClose: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 36) {
            Text("Using Cue TV").font(.largeTitle).accessibilityAddTraits(.isHeader)
            HStack(alignment: .top, spacing: 44) {
                section("Watch and resume", text: "Choose a catalog, title and episode, then select a source. Use Resume to continue or Restart to begin again. If a source cannot play, return to the source list and choose another.")
                section("Storage", text: "Current torrent limit: 4 GiB, with a 1 GiB free-space reserve. Choose a smaller source if it cannot fit. Media cache is disposable, so files may need to download again. Account and library storage are separate from media cache.")
                section("Torrent sharing", text: "Torrent playback can share downloaded pieces with other peers. Pausing the video does not end its transfer session. Close playback to stop that session; if cleanup fails, retry before starting another source.")
            }
            Text("Account recovery").font(.title2).accessibilityAddTraits(.isHeader)
            Text("If sync is paused, sign in again with the same Stremio account. Pending changes are kept. Retry failed sign-out cleanup before switching accounts.")
            Button("Close help", action: onClose).accessibilityIdentifier("close-help")
        }
        .padding(60)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(Color.black.ignoresSafeArea())
        .onExitCommand(perform: onClose)
    }

    private func section(_ title: String, text: String) -> some View {
        VStack(alignment: .leading, spacing: 20) {
            Text(title).font(.title2).accessibilityAddTraits(.isHeader)
            Text(text)
        }.frame(maxWidth: .infinity, alignment: .topLeading)
    }
}
