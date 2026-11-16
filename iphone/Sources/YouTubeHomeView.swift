import SwiftUI
import SafariServices

/// Safari owns the signed-in YouTube page. The app never reads its cookies or feed.
struct YouTubeHomeView: UIViewControllerRepresentable {
    @Environment(\.dismiss) private var dismiss
    func makeCoordinator() -> Coordinator { Coordinator(dismiss: { dismiss() }) }
    func makeUIViewController(context: Context) -> SFSafariViewController {
        let config = SFSafariViewController.Configuration()
        config.barCollapsingEnabled = false
        if let image = UIImage(systemName: "tv") {
            let button = SFSafariViewController.ActivityButton(templateImage: image,
                extensionIdentifier: "com.rupertlaycock.couchtv.remote.youtube-action")
            button.accessibilityLabel = "Play on TV"
            config.activityButton = button
        }
        // A constant destination, never a URL supplied by a remote web page.
        #if DEBUG
        let location = ProcessInfo.processInfo.arguments.contains("--youtube-test-video")
            ? "https://www.youtube.com/watch?v=jNQXAC9IVRw" : "https://www.youtube.com/"
        #else
        let location = "https://www.youtube.com/"
        #endif
        let url = URL(string: location)!
        let view = SFSafariViewController(url: url, configuration: config)
        view.dismissButtonStyle = .done
        view.delegate = context.coordinator
        return view
    }
    func updateUIViewController(_ controller: SFSafariViewController, context: Context) {}
    @MainActor final class Coordinator: NSObject, @preconcurrency SFSafariViewControllerDelegate {
        let dismiss: () -> Void
        init(dismiss: @escaping () -> Void) { self.dismiss = dismiss }
        func safariViewControllerDidFinish(_ controller: SFSafariViewController) { dismiss() }
    }
}
