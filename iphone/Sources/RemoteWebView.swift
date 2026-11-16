import SwiftUI
@preconcurrency import WebKit

struct RemoteWebView: UIViewRepresentable {
    @ObservedObject var model: RemoteModel
    func makeCoordinator() -> Coordinator { Coordinator(model: model) }
    func makeUIView(context: Context) -> RemoteSurfaceView {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .default()
        configuration.userContentController.add(context.coordinator, name: "couchNative")
        configuration.userContentController.addUserScript(WKUserScript(source: Self.bridgeScript, injectionTime: .atDocumentStart, forMainFrameOnly: true))
        let view = WKWebView(frame: .zero, configuration: configuration)
        view.accessibilityIdentifier = "tv-remote-web"
        view.navigationDelegate = context.coordinator; view.uiDelegate = context.coordinator
        view.isOpaque = false; view.backgroundColor = .black; view.scrollView.backgroundColor = .black
        view.scrollView.contentInsetAdjustmentBehavior = .never
        view.allowsBackForwardNavigationGestures = false
        model.webView = view
        let owner = model
        let surface = RemoteSurfaceView(remote: view, profileID: model.youtubeFeeds.selectedID) { [weak owner] video in
            guard let owner else { throw TVRemoteError.pairingRequired }
            try await owner.openYouTubeFeed(video)
        }
        context.coordinator.surface = surface
        model.surface = surface
        return surface
    }
    func updateUIView(_ surface: RemoteSurfaceView, context: Context) {
        surface.useProfile(model.youtubeFeeds.selectedID)
        guard model.loadRevision > 0, model.loadRevision != context.coordinator.loadedRevision, let url = model.pageURL else { return }
        context.coordinator.loadedRevision = model.loadRevision
        surface.feedFrame = nil
        surface.remote.load(URLRequest(url: url))
    }
    static func dismantleUIView(_ surface: RemoteSurfaceView, coordinator: Coordinator) {
        let view = surface.remote
        surface.feed.stop()
        view.stopLoading(); view.configuration.userContentController.removeScriptMessageHandler(forName: "couchNative")
        view.navigationDelegate = nil; view.uiDelegate = nil
    }

    @MainActor
    final class Coordinator: NSObject, WKNavigationDelegate, WKUIDelegate, WKScriptMessageHandler {
        let model: RemoteModel
        weak var surface: RemoteSurfaceView?
        var loadedRevision = 0
        init(model: RemoteModel) { self.model = model }
        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) { model.pageLoaded() }
        func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) { model.pageFailed() }
        func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) { model.pageFailed() }
        func webViewWebContentProcessDidTerminate(_ webView: WKWebView) { model.pageFailed() }
        func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction) async -> WKNavigationActionPolicy {
            guard let url = navigationAction.request.url else { return .cancel }
            if model.ownsPage(url) { return .allow }
            if url.scheme == "https" { _ = await UIApplication.shared.open(url) }
            return .cancel
        }
        func webView(_ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration, for navigationAction: WKNavigationAction, windowFeatures: WKWindowFeatures) -> WKWebView? {
            if let url = navigationAction.request.url {
                let localNotice = url.scheme == "http" && url.host == model.target.host && url.port == 5400
                    && ["/youtube/privacy.html", "/youtube/terms.html", "/youtube/index.html"].contains(url.path)
                if url.scheme == "https" || localNotice { UIApplication.shared.open(url) }
            }
            return nil
        }
        func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
            guard message.frameInfo.isMainFrame, let origin = message.frameInfo.request.url,
                  model.ownsPage(origin),
                  let body = message.body as? [String: Any], let action = body["action"] as? String else { return }
            if action == "paired", let token = body["token"] as? String { model.saveToken(token); return }
            if action == "youtubeHome" { surface?.feed.pause(); model.showingYouTube = true; return }
            if action == "youtubeFeedAccount" { surface?.feed.account(); return }
            if action == "youtubeManageFeeds" { surface?.feed.pause(); model.managingYouTubeFeeds = true; return }
            if action == "youtubeSelectFeed", let value = body["feed"] as? String, let id = UUID(uuidString: value) { model.selectYouTubeFeed(id); return }
            if action == "youtubeFeeds", let id = body["id"] as? String, UUID(uuidString: id) != nil {
                Task {
                    let reply: [String: Any] = ["id": id, "ok": true, "value": model.youtubeFeedsJSON()]
                    guard let data = try? JSONSerialization.data(withJSONObject: reply), let json = String(data: data, encoding: .utf8) else { return }
                    _ = try? await model.webView?.evaluateJavaScript("window.couchNative.complete(" + json + ")")
                }
                return
            }
            if action == "youtubeFeed" {
                surface?.feedFrame = (body["payload"] as? [String: Any]).flatMap(YouTubeFeedFrame.init)
                return
            }
            guard action == "wake", let id = body["id"] as? String, UUID(uuidString: id) != nil else { return }
            Task {
                var reply: [String: Any] = ["id": id]
                do { reply["ok"] = try await model.wake() }
                catch { reply["ok"] = false; reply["error"] = error.localizedDescription }
                guard let data = try? JSONSerialization.data(withJSONObject: reply), let json = String(data: data, encoding: .utf8) else { return }
                _ = try? await model.webView?.evaluateJavaScript("window.couchNative && window.couchNative.complete(" + json + ")")
            }
        }
    }

    private static let bridgeScript = #"""
    (function () {
      if (location.protocol !== 'http:' || location.port !== '5400') return;
      var pending = Object.create(null);
      window.couchNative = {
        canWake: true,
        canBrowseYouTube: true,
        canInlineYouTube: true,
        canManageYouTubeFeeds: true,
        manageYouTubeFeeds: function () { window.webkit.messageHandlers.couchNative.postMessage({action: 'youtubeManageFeeds'}); },
        selectYouTubeFeed: function (feed) { window.webkit.messageHandlers.couchNative.postMessage({action: 'youtubeSelectFeed', feed: feed}); },
        getYouTubeFeeds: function () {
          return new Promise(function (resolve, reject) {
            var id = crypto.randomUUID ? crypto.randomUUID() : 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) { var r = Math.random()*16|0; return (c === 'x' ? r : (r & 3 | 8)).toString(16); });
            var timer = setTimeout(function () { delete pending[id]; reject(new Error('Could not load YouTube feeds.')); }, 5000);
            pending[id] = {resolve: resolve, reject: reject, timer: timer};
            window.webkit.messageHandlers.couchNative.postMessage({action: 'youtubeFeeds', id: id});
          });
        },
        openYouTubeAccount: function () {
          window.webkit.messageHandlers.couchNative.postMessage({ action: 'youtubeFeedAccount' });
        },
        setYouTubeFeed: function (payload) {
          window.webkit.messageHandlers.couchNative.postMessage({ action: 'youtubeFeed', payload: payload });
        },
        openYouTubeHome: function () {
          window.webkit.messageHandlers.couchNative.postMessage({ action: 'youtubeHome' });
        },
        wake: function () {
          return new Promise(function (resolve, reject) {
            var id = crypto.randomUUID ? crypto.randomUUID() : 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) { var r = Math.random()*16|0; return (c === 'x' ? r : (r & 3 | 8)).toString(16); });
            var timer = setTimeout(function () { delete pending[id]; reject(new Error('The TV has not answered yet. Try again.')); }, 80000);
            pending[id] = { resolve: resolve, reject: reject, timer: timer };
            window.webkit.messageHandlers.couchNative.postMessage({ action: 'wake', id: id });
          });
        },
        complete: function (reply) {
          var p = pending[reply.id]; if (!p) return;
          clearTimeout(p.timer); delete pending[reply.id];
            if (reply.ok) p.resolve(reply.value || { ok: true }); else p.reject(new Error(reply.error || 'Could not reach the TV.'));
        }
      };
      var lastToken = '';
      setInterval(function () {
        var token = window.couchTv && window.couchTv.token && window.couchTv.token();
        if (token && token !== lastToken) { lastToken = token; window.webkit.messageHandlers.couchNative.postMessage({ action: 'paired', token: token }); }
      }, 1500);
    })();
    """#
}
