import SwiftUI

struct StandaloneHome: View {
    @State private var model = StandaloneModel()
    @State private var screen = Screen.catalogs
    @State private var manifest = ""
    @State private var search = ""
    @State private var restart = UUID()
    @State private var signIn = AccountSignInModel()
    @State private var showSignIn = false
    @State private var preparingAccount = false
    @State private var accountError: String?
    @State private var selectedSource: SelectedSource?
    enum Screen { case catalogs, titles, details, sources }
    struct SelectedSource: Identifiable { let id = UUID(); let request: CoreTorrentRequest }
    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Cue TV").font(.largeTitle)
            Text(model.snapshot?.model?.authenticated == true ? "Stremio · Account" : "Stremio · Guest").foregroundStyle(.secondary)
            if model.canSignOut {
                HStack {
                    Button(model.needsSignOutRetry ? "Retry sign-out" : "Sign out of Stremio") {
                        accountError = nil
                        Task {
                            do {
                                try await model.signOut()
                                screen = .catalogs
                                search = ""
                                manifest = ""
                                restart = UUID()
                            } catch { accountError = "Account cleanup could not finish. Please retry sign-out." }
                        }
                    }.disabled(model.signingOut)
                    Spacer()
                }.focusSection()
            } else {
                HStack {
                    Button("Sign in to Stremio") {
                        preparingAccount = true
                        accountError = nil
                        Task {
                            defer { preparingAccount = false }
                            guard await signIn.cancel() else { accountError = signIn.error; return }
                            do {
                                try await model.replace(with: nil)
                                signIn = AccountSignInModel()
                                showSignIn = true
                            } catch { accountError = "The current session could not stop. Please try again." }
                        }
                    }.disabled(preparingAccount)
                    Spacer()
                }.focusSection()
            }
            if let accountError { Text(accountError).foregroundStyle(.orange) }
            if let error = model.error { Text(error).foregroundStyle(.orange) }
            if let state = model.snapshot?.model {
                if state.error != nil { Text("The request could not be completed. Please retry.").foregroundStyle(.orange) }
                ScrollView {
                    VStack(alignment: .leading, spacing: 18) {
                        switch screen {
                        case .catalogs:
                            TextField("Search titles in a catalog", text: $search).autocorrectionDisabled()
                            ForEach(state.catalogs, id: \.index) { catalog in
                                Button("\(catalog.name) · \(catalog.kind)") {
                                    let query = search.trimmingCharacters(in: .whitespacesAndNewlines)
                                    Task {
                                        let admitted = await model.perform(expectedRevision: state.revision) { service, revision in
                                            if query.isEmpty { try await service.loadCatalog(index: catalog.index, expectedRevision: revision) }
                                            else { try await service.searchCatalog(index: catalog.index, query: query, expectedRevision: revision) }
                                        }
                                        if admitted { screen = .titles }
                                    }
                                }.disabled(!search.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && !catalog.searchable)
                            }
                            if state.catalogChoicesOffset > 0 { Button("Previous catalogs") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showCatalogChoices(offset: max(0, state.catalogChoicesOffset - 16), expectedRevision: $1) } } } }
                            if state.catalogChoicesOffset + 16 < state.catalogTotal { Button("More catalogs") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showCatalogChoices(offset: state.catalogChoicesOffset + 16, expectedRevision: $1) } } } }
                            TextField("Add-on manifest URL", text: $manifest).autocorrectionDisabled()
                            Button("Preview add-on") { let url = manifest; Task { await model.perform(expectedRevision: state.revision) { service, _ in try await service.loadAddon(url) } } }
                            if let addon = state.addon {
                                Text(addon.name ?? "Add-on")
                                if addon.status == .ready, !addon.installed { Button("Install add-on") { Task { await model.perform(expectedRevision: state.revision) { try await $0.installAddon(expectedRevision: $1) } } } }
                            }
                        case .titles:
                            if let catalog = state.catalog {
                                ForEach(Array(catalog.items.enumerated()), id: \.offset) { index, title in
                                    Button(title.name) { Task { if await model.perform(expectedRevision: state.revision, { try await $0.loadDetails(index: index, expectedRevision: $1) }) { screen = .details } } }
                                }
                                if catalog.status == .loading { ProgressView() }
                                if catalog.offset > 0 { Button("Previous titles") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showCatalogItems(offset: max(0, catalog.offset - 24), expectedRevision: $1) } } } }
                                if catalog.hasMore { Button("More titles") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showCatalogItems(offset: catalog.offset + catalog.items.count, expectedRevision: $1) } } } }
                            }
                        case .details:
                            if let details = state.details {
                                Text(details.name ?? "Title").font(.title)
                                if let description = details.description { Text(description) }
                                if details.status == .loading { ProgressView() }
                                if !details.seasons.isEmpty {
                                    ScrollView(.horizontal) {
                                        HStack {
                                            Button("All episodes") { Task { await model.perform(expectedRevision: state.revision) { try await $0.selectDetailsSeason(nil, expectedRevision: $1) } } }
                                                .disabled(details.selectedSeason == nil)
                                            ForEach(details.seasons, id: \.self) { season in
                                                Button(season == 0 ? "Specials" : "Season \(season)") { Task { await model.perform(expectedRevision: state.revision) { try await $0.selectDetailsSeason(season, expectedRevision: $1) } } }
                                                    .disabled(details.selectedSeason == season)
                                            }
                                        }.padding(.vertical, 12)
                                    }
                                }
                                ForEach(Array(details.videos.enumerated()), id: \.element.id) { index, video in
                                    Button(video.season.flatMap { season in video.episode.map { "S\(season) · E\($0) · \(video.title)" } } ?? video.title) { Task { if await model.perform(expectedRevision: state.revision, { try await $0.loadSources(videoIndex: index, expectedRevision: $1) }) { screen = .sources } } }
                                }
                                if details.kind == "movie", details.status == .ready, details.videoTotal == 0 { Button("Choose source") { Task { if await model.perform(expectedRevision: state.revision, { try await $0.loadSources(videoIndex: nil, expectedRevision: $1) }) { screen = .sources } } } }
                                if details.videoOffset > 0 { Button("Previous episodes") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showDetailsVideos(offset: max(0, details.videoOffset - 24), expectedRevision: $1) } } } }
                                if details.videoOffset + details.videos.count < details.videoTotal { Button("More episodes") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showDetailsVideos(offset: details.videoOffset + details.videos.count, expectedRevision: $1) } } } }
                            }
                        case .sources:
                            if let sources = state.sources {
                                if sources.loading { ProgressView("Finding sources…") }
                                ForEach(sources.items, id: \.index) { source in
                                    Button(source.name ?? "Source \(source.index + 1)") {
                                        Task { if let request = await model.prepare(index: source.index, expectedRevision: state.revision) { selectedSource = SelectedSource(request: request) } }
                                    }.disabled(source.kind != .torrent)
                                    if source.kind != .torrent { Text("This source format is not supported yet.").font(.caption) }
                                }
                                if sources.status == .ready, sources.items.isEmpty { Text("No sources found. Install a compatible Stremio add-on.") }
                                if sources.offset > 0 { Button("Previous sources") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showSources(offset: max(0, sources.offset - 24), expectedRevision: $1) } } } }
                                if sources.offset + sources.items.count < sources.total { Button("More sources") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showSources(offset: sources.offset + sources.items.count, expectedRevision: $1) } } } }
                            }
                        }
                    }
                }
                if screen != .catalogs { Button("Back") { screen = screen == .sources ? .details : screen == .details ? .titles : .catalogs } }
            } else if model.error != nil {
                Button("Retry Stremio") { screen = .catalogs; restart = UUID() }
            } else { ProgressView("Starting Stremio…") }
        }.padding(60)
        .task(id: restart) { await model.run() }
        .fullScreenCover(isPresented: $showSignIn) {
            AccountSignInView(model: signIn, home: model,
                onClose: { showSignIn = false; screen = .catalogs; restart = UUID() },
                onSuccess: { showSignIn = false; screen = .catalogs; restart = UUID() })
        }
        .fullScreenCover(item: $selectedSource) { TorrentPlaybackView(request: $0.request) }
    }
}
