import UIKit
@preconcurrency import WebKit

/// Geometry comes only from our paired TV's main page. YouTube has no native message handler.
struct YouTubeFeedFrame: Equatable {
    let full: CGRect
    let clip: CGRect
    let viewport: CGSize
    init?(payload: [String: Any]) {
        guard payload["visible"] as? Bool == true,
              let frame = payload["frame"] as? [String: Double],
              let visible = payload["clip"] as? [String: Double],
              let screen = payload["viewport"] as? [String: Double],
              let full = Self.rect(frame), let clip = Self.rect(visible),
              let width = screen["width"], let height = screen["height"],
              width.isFinite, height.isFinite, width >= 200, width <= 3000, height >= 200, height <= 3000,
              full.width <= width + 1, full.height <= 1600,
              clip.minX >= -1, clip.minY >= -1, clip.maxX <= width + 1, clip.maxY <= height + 1,
              full.intersection(clip).width >= clip.width - 1,
              full.intersection(clip).height >= clip.height - 1 else { return nil }
        self.full = full; self.clip = clip; viewport = CGSize(width: width, height: height)
    }
    private static func rect(_ values: [String: Double]) -> CGRect? {
        guard let x = values["x"], let y = values["y"], let width = values["width"], let height = values["height"],
              [x, y, width, height].allSatisfy(\.isFinite), abs(x) <= 6000, abs(y) <= 10000,
              width > 0, height > 0 else { return nil }
        return CGRect(x: x, y: y, width: width, height: height)
    }
}

@MainActor final class RemoteSurfaceView: UIView {
    let remote: WKWebView
    private(set) var feed: YouTubeInlineView
    private var profileID: UUID
    private let open: @MainActor (YouTubeVideo) async throws -> Void
    private let clipping = UIView()
    var feedFrame: YouTubeFeedFrame? { didSet { setNeedsLayout() } }
    init(remote: WKWebView, profileID: UUID, open: @escaping @MainActor (YouTubeVideo) async throws -> Void) {
        self.remote = remote; self.profileID = profileID; self.open = open
        self.feed = YouTubeInlineView(profileID: profileID, open: open)
        super.init(frame: .zero)
        clipsToBounds = true; clipping.clipsToBounds = true; clipping.isHidden = true
        addSubview(remote); addSubview(clipping); clipping.addSubview(feed)
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) is unavailable") }
    func useProfile(_ id: UUID) {
        guard id != profileID else { return }
        feed.stop(); feed.removeFromSuperview(); profileID = id
        feed = YouTubeInlineView(profileID: id, open: open)
        clipping.addSubview(feed); setNeedsLayout()
    }
    override func layoutSubviews() {
        super.layoutSubviews(); remote.frame = bounds
        guard let value = feedFrame else { clipping.isHidden = true; feed.pause(); return }
        let scale = bounds.width / value.viewport.width
        let visible = CGRect(x: value.clip.minX * scale, y: value.clip.minY * scale,
            width: value.clip.width * scale, height: value.clip.height * scale).intersection(bounds)
        guard !visible.isEmpty else { clipping.isHidden = true; feed.pause(); return }
        clipping.frame = visible; clipping.isHidden = false
        feed.frame = CGRect(x: value.full.minX * scale - visible.minX, y: value.full.minY * scale - visible.minY,
            width: value.full.width * scale, height: value.full.height * scale)
        feed.start()
    }
}

/// Displays YouTube's own website and per-phone WebKit session, without reading its feed or cookies.
@MainActor final class YouTubeInlineView: UIView, WKNavigationDelegate, WKUIDelegate {
    private let page: WKWebView
    private let spinner = UIActivityIndicatorView(style: .medium)
    private let status = UIStackView()
    private let message = UILabel()
    private let retry = UIButton(type: .system)
    private var windows: [YouTubeAccountWindow] = []
    private let open: @MainActor (YouTubeVideo) async throws -> Void
    private var observation: NSKeyValueObservation?
    private var operation: Task<Void, Never>?
    private var pending: YouTubeVideo?
    private var started = false
    private var returningHome = false
    private static let home = URL(string: "https://m.youtube.com/")!
    init(profileID: UUID, open: @escaping @MainActor (YouTubeVideo) async throws -> Void) {
        self.open = open
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = WKWebsiteDataStore(forIdentifier: profileID)
        page = WKWebView(frame: .zero, configuration: configuration)
        super.init(frame: .zero)
        page.accessibilityIdentifier = "youtube-inline-web"
        clipsToBounds = true; layer.cornerRadius = 14; backgroundColor = .black
        page.navigationDelegate = self; page.uiDelegate = self
        page.isOpaque = false; page.backgroundColor = .black; page.scrollView.backgroundColor = .black
        page.scrollView.contentInsetAdjustmentBehavior = .never
        page.allowsBackForwardNavigationGestures = false
        addSubview(page); addSubview(spinner)
        message.font = .preferredFont(forTextStyle: .footnote); message.numberOfLines = 3
        retry.setTitle("Try again", for: .normal); retry.addTarget(self, action: #selector(tryAgain), for: .touchUpInside)
        status.axis = .horizontal; status.spacing = 12; status.alignment = .center
        status.isLayoutMarginsRelativeArrangement = true; status.layoutMargins = UIEdgeInsets(top: 10, left: 12, bottom: 10, right: 12)
        status.backgroundColor = .secondarySystemBackground; status.layer.cornerRadius = 10
        status.addArrangedSubview(message); status.addArrangedSubview(retry); status.isHidden = true; addSubview(status)
        observation = page.observe(\.url, options: [.new]) { [weak self] _, _ in
            Task { @MainActor [weak self] in self?.observeAddress() }
        }
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) is unavailable") }
    override func layoutSubviews() {
        super.layoutSubviews(); page.frame = bounds; spinner.center = CGPoint(x: bounds.midX, y: bounds.midY)
        status.frame = CGRect(x: 8, y: 8, width: max(0, bounds.width - 16), height: 76)
    }
    func start() {
        guard !started else { return }; started = true
        #if DEBUG
        if ProcessInfo.processInfo.environment["COUCHTV_REMOTE_PREVIEW"] == "1",
           ProcessInfo.processInfo.environment["COUCHTV_YOUTUBE_POPUP_PREVIEW"] == "1" {
            page.loadHTMLString(Self.popupPreview, baseURL: Self.home); return
        }
        #endif
        page.load(URLRequest(url: Self.home))
    }
    func account() {
        dismissWindows()
        started = true; returningHome = false; status.isHidden = true
        page.load(URLRequest(url: URL(string: "https://m.youtube.com/feed/you")!))
    }
    func pause() {
        endEditing(true)
        for view in [page] + windows.map(\.page) {
            view.endEditing(true)
            guard let host = view.url?.host, host == "youtube.com" || host.hasSuffix(".youtube.com") else { continue }
            view.evaluateJavaScript("document.querySelectorAll('video').forEach(function(video){video.pause();});", completionHandler: nil)
        }
    }
    func stop() {
        dismissWindows()
        operation?.cancel(); operation = nil; observation = nil
        page.stopLoading(); page.navigationDelegate = nil; page.uiDelegate = nil
    }
    private func observeAddress() {
        guard !returningHome, let url = page.url, let video = YouTubeVideo(url: url) else { return }
        select(video)
    }
    private func select(_ video: YouTubeVideo) {
        guard operation == nil else { return }
        dismissWindows()
        pending = video; returningHome = true; page.stopLoading(); pause()
        page.load(URLRequest(url: Self.home))
        message.text = "Opening on your TV…"; retry.isHidden = true; status.isHidden = false
        operation = Task { [weak self] in
            guard let self else { return }
            defer { operation = nil; retry.isHidden = false }
            do { try await open(video); status.isHidden = true; pending = nil }
            catch is CancellationError { }
            catch {
                if let remoteError = error as? TVRemoteError, case .pairingRequired = remoteError { message.text = remoteError.localizedDescription }
                else { message.text = "Couldn’t open this video on the TV. Try again." }
            }
        }
    }
    @objc private func tryAgain() {
        status.isHidden = true
        if let pending { select(pending) } else { (windows.last?.page ?? page).reload() }
    }
    func webView(_ webView: WKWebView, decidePolicyFor action: WKNavigationAction) async -> WKNavigationActionPolicy {
        guard let url = action.request.url else { return .cancel }
        if action.targetFrame?.isMainFrame == false {
            return YouTubeNavigation.allowsSubframe(url) ? .allow : .cancel
        }
        if url.absoluteString == "about:blank",
           windows.contains(where: { $0.page === webView }) || (action.targetFrame == nil && YouTubeNavigation.isServiceOrigin(action.sourceFrame.securityOrigin)) { return .allow }
        guard YouTubeNavigation.isHTTPS(url) else { return .cancel }
        if let video = YouTubeVideo(url: url) { select(video); return .cancel }
        if YouTubeNavigation.isService(url) { return .allow }
        if action.navigationType == .linkActivated { _ = await UIApplication.shared.open(url) }
        else {
            spinner.stopAnimating(); message.text = "YouTube couldn’t finish this navigation. Try again or use YouTube Home’s browser."; retry.isHidden = false; status.isHidden = false
        }
        return .cancel
    }
    func webView(_ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration,
                 for action: WKNavigationAction, windowFeatures: WKWindowFeatures) -> WKWebView? {
        guard action.targetFrame == nil, let url = action.request.url,
              windows.count < 4,
              YouTubeNavigation.isService(url) || (url.absoluteString == "about:blank" && YouTubeNavigation.isServiceOrigin(action.sourceFrame.securityOrigin)) else { return nil }
        guard var presenter = hostingController else { return nil }
        while let presented = presenter.presentedViewController { presenter = presented }
        // Use WebKit's supplied configuration so the original opener and feed session survive.
        let window = WKWebView(frame: .zero, configuration: configuration)
        window.accessibilityIdentifier = "youtube-signin-window"
        window.navigationDelegate = self; window.uiDelegate = self
        window.scrollView.contentInsetAdjustmentBehavior = .never
        let account = YouTubeAccountWindow(page: window)
        account.onClose = { [weak self, weak account] in
            guard let self, let account else { return }
            self.webViewDidClose(account.page)
        }
        windows.append(account)
        let navigation = UINavigationController(rootViewController: account)
        navigation.modalPresentationStyle = .fullScreen
        presenter.present(navigation, animated: true)
        return window
    }
    private var hostingController: UIViewController? {
        window?.rootViewController
    }
    func webViewDidClose(_ webView: WKWebView) {
        guard let index = windows.firstIndex(where: { $0.page === webView }) else { return }
        while windows.count > index { removeLastWindow() }
    }
    private func removeLastWindow() {
        guard let account = windows.popLast() else { return }
        account.page.stopLoading(); account.page.navigationDelegate = nil; account.page.uiDelegate = nil
        account.navigationController?.dismiss(animated: false)
    }
    private func dismissWindows() { while !windows.isEmpty { removeLastWindow() } }
    func webView(_ webView: WKWebView, didStartProvisionalNavigation navigation: WKNavigation!) { spinner.startAnimating() }
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) { spinner.stopAnimating(); returningHome = false; observeAddress() }
    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) { failed(error) }
    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) { failed(error) }
    private func failed(_ error: Error) {
        spinner.stopAnimating()
        guard (error as NSError).code != NSURLErrorCancelled, operation == nil else { return }
        returningHome = false; message.text = "Couldn’t load YouTube Home."; retry.isHidden = false; status.isHidden = false
    }
    #if DEBUG
    private static let popupPreview = #"""
    <!doctype html><meta name="viewport" content="width=device-width, initial-scale=1">
    <body style="background:#111;color:white;font:20px system-ui;padding:20px">
    <h1>Sign-in window test</h1><p id="result">Waiting for test sign-in</p>
    <button style="font:inherit;padding:16px" onclick="openSignIn()">Open test sign-in</button>
    <script>
    function openSignIn() {
      const child = window.open('about:blank', 'sign-in');
      child.document.write(`<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1"><body style="background:#111;color:white;font:20px system-ui;padding:20px"><h1>Test account window</h1><button style="font:inherit;padding:16px" onclick="window.opener.document.getElementById('result').textContent='Test sign-in returned';window.close();">Complete test sign-in</button>`);
      child.document.close();
    }
    </script>
    """#
    #endif
}

enum YouTubeNavigation {
    static func isHTTPS(_ url: URL) -> Bool {
        url.scheme?.lowercased() == "https" && url.host?.isEmpty == false && url.user == nil && url.password == nil && (url.port == nil || url.port == 443)
    }
    static func isService(_ url: URL) -> Bool { isHTTPS(url) && isServiceHost(url.host) }
    static func isServiceOrigin(_ origin: WKSecurityOrigin) -> Bool {
        origin.protocol == "https" && (origin.port == 0 || origin.port == 443) && isServiceHost(origin.host)
    }
    private static func isServiceHost(_ host: String?) -> Bool {
        guard let host = host?.lowercased() else { return false }
        return host == "youtube.com" || host.hasSuffix(".youtube.com") || host == "google.com" || host.hasSuffix(".google.com")
    }
    static func allowsSubframe(_ url: URL) -> Bool {
        if isHTTPS(url) { return true }
        return ["about:blank", "about:srcdoc"].contains(url.absoluteString) || ["data", "blob"].contains(url.scheme?.lowercased() ?? "")
    }
}

/// Account windows use a full-screen native presentation so forms and the keyboard stay accessible.
@MainActor private final class YouTubeAccountWindow: UIViewController {
    let page: WKWebView
    var onClose: (() -> Void)?
    init(page: WKWebView) { self.page = page; super.init(nibName: nil, bundle: nil) }
    required init?(coder: NSCoder) { fatalError("init(coder:) is unavailable") }
    override func viewDidLoad() {
        super.viewDidLoad()
        title = "YouTube account"
        view.backgroundColor = .systemBackground
        navigationItem.leftBarButtonItem = UIBarButtonItem(title: "Back to YouTube", style: .plain, target: self, action: #selector(close))
        page.translatesAutoresizingMaskIntoConstraints = false; view.addSubview(page)
        NSLayoutConstraint.activate([
            page.leadingAnchor.constraint(equalTo: view.leadingAnchor), page.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            page.topAnchor.constraint(equalTo: view.safeAreaLayoutGuide.topAnchor), page.bottomAnchor.constraint(equalTo: view.bottomAnchor)
        ])
    }
    @objc private func close() { onClose?() }
}
