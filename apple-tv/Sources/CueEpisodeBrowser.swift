import SwiftUI

nonisolated struct CueEpisode: Decodable, Identifiable, Sendable {
    let id: String
    let season: Int
    let episode: Int
    let title: String
    let released: String?
    var available: Bool {
        guard let released else { return true }
        let formatter = ISO8601DateFormatter()
        let ordinary = formatter.date(from: released)
        formatter.formatOptions.insert(.withFractionalSeconds)
        guard let date = ordinary ?? formatter.date(from: released) else { return true }
        return date <= Date()
    }
}
nonisolated struct CueTitleInfo: Decodable, Sendable { let videos: [CueEpisode] }
nonisolated struct CueSource: Decodable, Identifiable, Sendable {
    let id: String
    let name: String
    let provider: String
    let description: String
    let playable: Bool
    let reason: String?
}
nonisolated struct CueSources: Decodable, Sendable { let items: [CueSource]; let failed: Int? }

struct CueEpisodeBrowser: View {
    let item: Title
    let service: TVService
    @State private var info: CueTitleInfo?
    @State private var season = 1
    @State private var error: String?
    @Environment(\.dismiss) private var dismiss
    init(item: Title, service: TVService) {
        self.item = item; self.service = service
        let parts = item.video?.split(separator: ":") ?? []
        _season = State(initialValue: parts.count >= 3 ? Int(parts[parts.count - 2]) ?? 1 : 1)
    }
    var body: some View {
        NavigationStack {
            Group {
                if item.type == "movie" { CueSourceBrowser(item: item, video: item.id, service: service) }
                else if let info {
                    List {
                        Section {
                            Picker("Season", selection: $season) {
                                ForEach(Array(Set(info.videos.map(\.season))).sorted(), id: \.self) { Text($0 == 0 ? "Specials" : "Season \($0)").tag($0) }
                            }
                        }
                        Section("Episodes") {
                            ForEach(info.videos.filter { $0.season == season }) { episode in
                                NavigationLink { CueSourceBrowser(item: item, video: episode.id, service: service) } label: {
                                    VStack(alignment: .leading, spacing: 8) {
                                        Text("\(episode.episode). \(episode.title)").font(.headline)
                                        if !episode.available { Text("Not released yet").font(.callout).foregroundStyle(.secondary) }
                                    }.padding(.vertical, 10)
                                }.disabled(!episode.available)
                            }
                        }
                    }
                } else if let error {
                    VStack(spacing: 24) { Text(error); Button("Try again") { Task { await load() } }; Button("Back") { dismiss() } }
                } else { ProgressView("Loading episodes…") }
            }
            .navigationTitle(item.title)
            .task { if item.type == "series" { await load() } }
        }
    }
    private func load() async {
        error = nil
        do {
            info = try await service.request("api/stremio/detail", as: CueTitleInfo.self, parameters: ["type": item.type, "id": item.id])
            if !(info?.videos.contains { $0.season == season } ?? false) { season = info?.videos.first?.season ?? 1 }
        } catch { self.error = "Couldn’t load these episodes. Try again." }
    }
}

private struct CueSourceBrowser: View {
    let item: Title
    let video: String
    let service: TVService
    @State private var sources: CueSources?
    @State private var error: String?
    @State private var busy: String?
    var body: some View {
        Group {
            if let sources {
                List {
                    if let error { Section { Text(error).foregroundStyle(.orange) } }
                    if sources.items.isEmpty { Section { Text("No Real-Debrid Torrentio sources are available for this episode."); Button("Try again") { Task { await load() } } } }
                    ForEach(sources.items) { source in
                        Button { Task { await play(source) } } label: {
                            HStack(spacing: 24) {
                                Image(systemName: "play.fill")
                                VStack(alignment: .leading, spacing: 8) {
                                    Text(source.name).font(.headline)
                                    Text(source.description).font(.callout).lineLimit(3).foregroundStyle(.secondary)
                                    if !source.playable { Text(source.reason ?? "Choose another source.").font(.callout) }
                                }
                                Spacer()
                                if busy == source.id { ProgressView() }
                            }.padding(.vertical, 12)
                        }.disabled(!source.playable || busy != nil)
                    }
                }
            } else if let error { VStack(spacing: 24) { Text(error); Button("Try again") { Task { await load() } } } }
            else { ProgressView("Finding your Real-Debrid sources…") }
        }.navigationTitle("Choose a source").task { await load() }
    }
    private func load() async {
        error = nil
        do { sources = try await service.request("api/stremio/sources", as: CueSources.self, parameters: ["type": item.type, "id": item.id, "video": video]) }
        catch { self.error = "Couldn’t load sources. Try again." }
    }
    private func play(_ source: CueSource) async {
        guard busy == nil else { return }; busy = source.id; error = nil
        defer { busy = nil }
        do {
            let media = try await service.cueChoice(source.id)
            if !CuePlayerPresenter.show(media, service: service) { error = "This source couldn’t open. Try another source." }
        } catch { self.error = "Couldn’t open this source. Try again or choose another." }
    }
}
