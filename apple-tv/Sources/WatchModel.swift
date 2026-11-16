import Foundation
import Observation
import UIKit

@Observable final class WatchModel {
    var catalogue = Catalogue()
    var collections: [CollectionShelf] = []
    var library: [Title] = []
    var linked = false
    var connected = false
    var loading = false
    var pairing = false
    var error: String?
    var address = ""
    var code = ""
    var selected: Title?
    var homeRequest = 0
    private var lastHomeSignal: Int64?
    private var lastTitleSignal: Int64?
    let service: TVService

    init() {
        let saved = ConnectionKeychain.load()
        service = TVService(connection: saved)
        connected = saved != nil
        address = saved?.address ?? Self.defaultAddress()
        #if DEBUG
        if let fixture = ProcessInfo.processInfo.environment["COUCHTV_PREVIEW_URL"], TVAddress.normalise(fixture) != nil {
            service.connection = TVConnection(address: fixture, token: "preview-only")
            connected = true; address = fixture
        }
        #endif
    }
    private static func defaultAddress() -> String {
        guard let path = Bundle.main.url(forResource: "TVConfig", withExtension: "plist"),
              let data = try? Data(contentsOf: path),
              let config = try? PropertyListSerialization.propertyList(from: data, format: nil) as? [String: Any],
              let host = config["Host"] as? String else { return "" }
        return "http://\(host):5400"
    }
    func connect() async {
        guard !pairing else { return }
        pairing = true; error = nil
        defer { pairing = false }
        do {
            let connection = try await service.pair(address: address, code: code)
            try ConnectionKeychain.save(connection)
            service.connection = connection; connected = true; code = ""
            await refresh()
        } catch { self.error = error.localizedDescription }
    }
    func accept(_ url: URL) async {
        if url.scheme == "couchtv", url.host == "home" {
            selected = nil; homeRequest += 1
            return
        }
        guard url.scheme == "couchtv", url.host == "pair", !connected,
              let parts = URLComponents(url: url, resolvingAgainstBaseURL: false),
              let host = parts.queryItems?.first(where: { $0.name == "address" })?.value,
              let pin = parts.queryItems?.first(where: { $0.name == "code" })?.value,
              TVAddress.normalise(host) != nil else { return }
        address = host; code = pin
        await connect()
    }
    func refresh() async {
        guard connected, !loading else { return }
        loading = true; error = nil
        defer { loading = false }
        do { catalogue = try await service.request("api/stremio/discover", as: Catalogue.self) }
        catch { handle(error) }
        do {
            let result = try await service.request("api/stremio/library", as: Library.self)
            library = result.items; linked = result.connected
        } catch { handle(error) }
        do { collections = try await service.request("api/stremio/collections", as: ServiceCollections.self).collections }
        catch { /* Existing catalogue and history remain usable during a provider outage. */ }
    }
    func observeRemote() async {
        await service.events { [weak self] event in
            guard let self else { return }
            if let revision = event.platform?.home_request {
                if revision > 0 && revision != lastHomeSignal { selected = nil; homeRequest += 1 }
                lastHomeSignal = revision
            }
            if let request = event.platform?.open_request, request.serial != lastTitleSignal {
                lastTitleSignal = request.serial
                if request.service == "stremio-player" {
                    Task { await openStremioPlayer(request) }
                    return
                }
                if (request.service ?? "stremio") == "stremio" {
                    Task {
                        do {
                            selected = try await service.request("api/stremio/detail", as: Title.self, parameters: ["type": request.type, "id": request.id])
                            try await service.confirmTitle(request.serial, opened: true)
                        } catch { self.error = "Couldn’t open this title. Try again."; try? await service.confirmTitle(request.serial, opened: false) }
                    }
                    return
                }
                selected = nil
                guard let url = request.url else {
                    Task { try? await service.confirmTitle(request.serial, opened: false) }
                    return
                }
                let app = request.service == "youtube" ? "YouTube" : "Stremio"
                openURL(url, missing: "Install \(app) on Apple TV, then try again.", replyTo: request.serial)
            }
        }
    }
    private func openStremioPlayer(_ request: TVEvent.TitleRequest) async {
        guard request.type == "stream", request.id.range(of: "^[a-f0-9]{32}$", options: .regularExpression) != nil else {
            try? await service.confirmTitle(request.serial, opened: false)
            return
        }
        do {
            let link = try await service.request("api/stremio/playback", as: StremioPlayerLink.self, parameters: ["id": request.id])
            guard let media = link.media else { throw TVError(message: "Update Cue TV on the Android host to use its internal player.", status: 409) }
            let opened = CuePlayerPresenter.show(media, service: service)
            try await service.confirmTitle(request.serial, opened: opened)
            if !opened { error = "This source couldn’t be opened. Choose another source." }
        } catch {
            self.error = "Couldn’t receive this Stremio source. Choose it again on your phone."
            try? await service.confirmTitle(request.serial, opened: false)
        }
    }
    private func handle(_ error: any Error) {
        self.error = error.localizedDescription
        if (error as? TVError)?.status == 401 { connected = false }
    }
    func open(_ item: Title) {
        guard let url = item.stremioURL else { error = "This title could not be opened."; return }
        openURL(url, missing: "Install Stremio on Apple TV, then try this title again.")
    }
    func youtube() { launch("youtube") }
    func stremio() { launch("stremio") }
    private func launch(_ app: String) {
        Task {
            do { try await service.launch(app) }
            catch { self.error = error.localizedDescription }
        }
    }
    private func openURL(_ url: URL, missing: String, replyTo serial: Int64? = nil) {
        UIApplication.shared.open(url) { [weak self] opened in
            if !opened { self?.error = missing }
            if let self, let serial { Task { try? await service.confirmTitle(serial, opened: opened) } }
        }
    }
}
