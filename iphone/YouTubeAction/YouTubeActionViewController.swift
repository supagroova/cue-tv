import SwiftUI
import UniformTypeIdentifiers

@MainActor
final class YouTubeActionViewController: UIViewController {
    private let state = YouTubeActionState()
    private var operation: Task<Void, Never>?
    private var completed = false
    override func viewDidLoad() {
        super.viewDidLoad()
        let child = UIHostingController(rootView: YouTubeActionScreen(state: state,
            retry: { [weak self] in self?.start() }, close: { [weak self] in self?.finish(played: false) }))
        addChild(child); view.addSubview(child.view)
        child.view.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([child.view.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            child.view.trailingAnchor.constraint(equalTo: view.trailingAnchor), child.view.topAnchor.constraint(equalTo: view.topAnchor),
            child.view.bottomAnchor.constraint(equalTo: view.bottomAnchor)])
        child.didMove(toParent: self)
        preferredContentSize = CGSize(width: 390, height: 300)
        start()
    }
    private func start() {
        guard operation == nil, !completed else { return }
        state.message = nil; state.success = false
        operation = Task { [weak self] in
            guard let self else { return }
            defer { operation = nil }
            do {
                let url = try await pageURL()
                guard let video = YouTubeVideo(url: url) else { throw YouTubeHandoffError.chooseVideo }
                let store = PairingStore()
                guard let target = store.currentTarget(), let token = store.token(for: target) else { throw TVRemoteError.pairingRequired }
                state.tvName = target.name
                try await YouTubePlaybackClient().open(video, target: target, token: token)
                try Task.checkCancellation()
                state.success = true
                try await Task.sleep(for: .milliseconds(700))
                finish(played: true)
            } catch is CancellationError { }
            catch { state.message = error.localizedDescription }
        }
    }
    private func pageURL() async throws -> URL {
        let items = extensionContext?.inputItems.compactMap { $0 as? NSExtensionItem } ?? []
        for item in items {
            for attachment in item.attachments ?? [] {
                if attachment.hasItemConformingToTypeIdentifier(UTType.propertyList.identifier),
                   let value = try? await loadURL(attachment, type: UTType.propertyList.identifier), let url = URL(string: value) { return url }
                if attachment.hasItemConformingToTypeIdentifier(UTType.url.identifier),
                   let value = try? await loadURL(attachment, type: UTType.url.identifier), let url = URL(string: value) { return url }
            }
        }
        throw YouTubeHandoffError.chooseVideo
    }
    private func loadURL(_ provider: NSItemProvider, type: String) async throws -> String {
        try await withCheckedThrowingContinuation { continuation in
            provider.loadItem(forTypeIdentifier: type, options: nil) { value, error in
                if let error { continuation.resume(throwing: error); return }
                let page = (value as? NSDictionary)?[NSExtensionJavaScriptPreprocessingResultsKey] as? [String: Any]
                let string = page?["url"] as? String ?? (value as? URL)?.absoluteString ?? (value as? String)
                if let string { continuation.resume(returning: string) }
                else { continuation.resume(throwing: YouTubeHandoffError.chooseVideo) }
            }
        }
    }
    private func finish(played: Bool) {
        guard !completed else { return }; completed = true
        operation?.cancel()
        let item = NSExtensionItem()
        let result: NSDictionary = [NSExtensionJavaScriptFinalizeArgumentKey: ["played": played]]
        item.attachments = [NSItemProvider(item: result, typeIdentifier: UTType.propertyList.identifier)]
        extensionContext?.completeRequest(returningItems: [item], completionHandler: nil)
    }
}

@MainActor
final class YouTubeActionState: ObservableObject {
    @Published var message: String?
    @Published var success = false
    @Published var tvName = "your TV"
}

private struct YouTubeActionScreen: View {
    @ObservedObject var state: YouTubeActionState
    let retry: () -> Void
    let close: () -> Void
    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: state.success ? "checkmark.circle" : "tv").font(.system(size: 36)).accessibilityHidden(true)
            Text(state.success ? "Opened on TV" : "Play on TV").font(.title2.bold())
            if let message = state.message {
                Text(message).multilineTextAlignment(.center).foregroundStyle(.secondary)
                HStack { Button("Close", action: close).frame(minHeight: 44); Spacer(); Button("Try again", action: retry).frame(minHeight: 44) }
            } else if !state.success {
                ProgressView("Opening on \(state.tvName)…")
                Button("Cancel", action: close).frame(minHeight: 44)
            }
        }.padding(28).frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
