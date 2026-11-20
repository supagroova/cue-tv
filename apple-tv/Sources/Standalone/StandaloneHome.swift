import SwiftUI

struct StandaloneHome: View {
    @State private var model: StandaloneModel
    init(model: StandaloneModel = StandaloneModel()) { _model = State(initialValue: model) }
    @State private var screen = Screen.catalogs
    @State private var manifest = ""
    @State private var search = ""
    @State private var restart = UUID()
    @State private var signIn = AccountSignInModel()
    @State private var showSignIn = false
    @State private var preparingAccount = false
    @State private var accountError: String?
    @State private var selectedSource: SelectedSource?
    @State private var detailsOrigin = Screen.titles
    enum Screen: Hashable { case catalogs, titles, library, details, sources }
    enum Row: Hashable {
        case libraryHome, catalog(Int), title(String, String), video(String), movie(String), source(String, Int)
    }
    @FocusState private var focusedRow: Row?
    @State private var rememberedRows: [Screen: Row] = [:]
    private var backAction: (() -> Void)? { screen == .catalogs ? nil : { goBack() } }
    private func goBack() {
        switch screen {
        case .sources: screen = .details
        case .details: screen = detailsOrigin
        case .titles, .library: screen = .catalogs
        case .catalogs: break
        }
    }
    struct SelectedSource: Identifiable { let id = UUID(); let request: CorePlaybackRequest }
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
                if let sync = state.sync {
                    switch sync.state {
                    case .idle: EmptyView()
                    case .syncing:
                        Text("Syncing \(sync.pendingTitles) pending titles…").foregroundStyle(.secondary)
                    case .waiting:
                        HStack {
                            Text("\(sync.pendingTitles) titles are waiting to sync.").foregroundStyle(.orange)
                            Button("Retry sync") { Task { await model.retrySync() } }
                                .disabled(model.retryingSync || model.signingOut)
                        }.focusSection()
                    case .reauthenticationRequired:
                        Text("Stremio rejected this account session. Sync is paused; pending changes are retained.")
                            .foregroundStyle(.orange)
                    }
                }
                if let error = state.error, state.sync == nil || !error.hasPrefix("account_sync_") {
                    Text("The request could not be completed. Please retry.").foregroundStyle(.orange)
                }
                ScrollView {
                    VStack(alignment: .leading, spacing: 18) {
                        switch screen {
                        case .catalogs:
                            Button("My library") {
                                rememberedRows[.catalogs] = .libraryHome
                                Task { if await model.perform(expectedRevision: state.revision, { try await $0.showLibrary(offset: state.library?.offset ?? 0, expectedRevision: $1) }) { screen = .library } }
                            }.focused($focusedRow, equals: .libraryHome)
                            TextField("Search titles in a catalog", text: $search).autocorrectionDisabled()
                            ForEach(state.catalogs, id: \.index) { catalog in
                                Button("\(catalog.name) · \(catalog.kind)") {
                                    rememberedRows[.catalogs] = .catalog(catalog.index)
                                    let query = search.trimmingCharacters(in: .whitespacesAndNewlines)
                                    Task {
                                        let admitted = await model.perform(expectedRevision: state.revision) { service, revision in
                                            if query.isEmpty { try await service.loadCatalog(index: catalog.index, expectedRevision: revision) }
                                            else { try await service.searchCatalog(index: catalog.index, query: query, expectedRevision: revision) }
                                        }
                                        if admitted { screen = .titles }
                                    }
                                }.focused($focusedRow, equals: .catalog(catalog.index))
                                .disabled(!search.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && !catalog.searchable)
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
                                ForEach(Array(catalog.items.enumerated()), id: \.element.id) { index, title in
                                    Button(title.name) { rememberedRows[.titles] = .title(title.kind, title.id); Task { if await model.perform(expectedRevision: state.revision, { try await $0.loadDetails(index: index, expectedRevision: $1) }) { detailsOrigin = .titles; screen = .details } } }
                                    .focused($focusedRow, equals: .title(title.kind, title.id))
                                }
                                if catalog.status == .loading { ProgressView() }
                                if catalog.status == .failed {
                                    Text("Titles could not be loaded. Try again.").foregroundStyle(.orange)
                                    Button("Retry titles") {
                                        Task { await model.perform(expectedRevision: state.revision) { try await $0.showCatalogItems(offset: catalog.offset, expectedRevision: $1) } }
                                    }
                                }
                                if catalog.status == .ready, catalog.items.isEmpty, !catalog.hasMore {
                                    Text("No titles found. Go back to change your search or choose another catalog.")
                                }
                                if catalog.offset > 0 { Button("Previous titles") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showCatalogItems(offset: max(0, catalog.offset - 24), expectedRevision: $1) } } } }
                                if catalog.hasMore { Button("More titles") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showCatalogItems(offset: catalog.offset + catalog.items.count, expectedRevision: $1) } } } }
                            }
                        case .library:
                            Text("My library").font(.title)
                            if let library = state.library {
                                ForEach(Array(library.items.enumerated()), id: \.element.id) { index, title in
                                    Button(title.name) {
                                        rememberedRows[.library] = .title(title.kind, title.id)
                                        Task { if await model.perform(expectedRevision: state.revision, { try await $0.loadLibraryDetails(index: index, expectedRevision: $1) }) { detailsOrigin = .library; screen = .details } }
                                    }.focused($focusedRow, equals: .title(title.kind, title.id))
                                }
                                if library.total == 0 { Text("Your library is empty. Add a title from its details.") }
                                if library.offset > 0 {
                                    Button("Previous saved titles") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showLibrary(offset: max(0, library.offset - 24), expectedRevision: $1) } } }
                                }
                                if library.offset + library.items.count < library.total {
                                    Button("More saved titles") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showLibrary(offset: library.offset + library.items.count, expectedRevision: $1) } } }
                                }
                            }
                        case .details:
                            if let details = state.details {
                                Text(details.name ?? "Title").font(.title)
                                if let description = details.description { Text(description) }
                                if details.status == .loading { ProgressView() }
                                if details.status == .ready, let member = state.detailsInLibrary {
                                    LibraryMembershipControl(member: member) { desired in
                                        await model.perform(expectedRevision: state.revision) { service, revision in
                                            try await service.setDetailsLibrary(desired, expectedRevision: revision)
                                        }
                                    }.id(details.kind + "\n" + details.id)
                                }

                                if details.status == .ready, let watched = details.watched {
                                    WatchedStateControl(watched: watched) { desired in
                                        await model.perform(expectedRevision: state.revision) { service, revision in
                                            try await service.setDetailsWatched(desired, videoID: nil, expectedRevision: revision)
                                        }
                                    }.id("watched\n" + details.kind + "\n" + details.id)
                                }

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
                                VStack(alignment: .leading, spacing: 18) {
                                    ForEach(Array(details.videos.enumerated()), id: \.element.id) { index, video in
                                        HStack(spacing: 24) {
                                            Button(video.season.flatMap { season in video.episode.map { "S\(season) · E\($0) · \(video.title)" } } ?? video.title) { rememberedRows[.details] = .video(video.id); Task { if await model.perform(expectedRevision: state.revision, { try await $0.loadSources(videoIndex: index, expectedRevision: $1) }) { screen = .sources } } }
                                            .focused($focusedRow, equals: .video(video.id))
                                            if details.kind == "series", let watched = video.watched {
                                                WatchedStateControl(watched: watched, subject: video.season.flatMap { season in video.episode.map { "S\(season) E\($0)" } } ?? video.title) { desired in
                                                    await model.perform(expectedRevision: state.revision) { service, revision in
                                                        try await service.setDetailsWatched(desired, videoID: video.id, expectedRevision: revision)
                                                    }
                                                }.id(details.kind + "\n" + details.id + "\n" + video.id)
                                            }
                                        }
                                    }
                                    if details.kind == "movie", details.status == .ready, details.videoTotal == 0 { Button("Choose source") { rememberedRows[.details] = .movie(details.id); Task { if await model.perform(expectedRevision: state.revision, { try await $0.loadSources(videoIndex: nil, expectedRevision: $1) }) { screen = .sources } } }.focused($focusedRow, equals: .movie(details.id)) }
                                    if details.videoOffset > 0 { Button("Previous episodes") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showDetailsVideos(offset: max(0, details.videoOffset - 24), expectedRevision: $1) } } } }
                                    if details.videoOffset + details.videos.count < details.videoTotal { Button("More episodes") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showDetailsVideos(offset: details.videoOffset + details.videos.count, expectedRevision: $1) } } } }
                                }.frame(maxWidth: .infinity, alignment: .leading).focusSection()
                                .onChange(of: details.selectedSeason) { _, _ in
                                    // The selected season button becomes disabled. Keep the next
                                    // remote movement in the episode column, not the watched controls.
                                    if let first = details.videos.first { focusedRow = .video(first.id) }
                                }
                            }
                        case .sources:
                            if let sources = state.sources {
                                if sources.loading { ProgressView("Finding sources…") }
                                ForEach(sources.groups) { group in
                                    Text(group.name).font(.headline)
                                    ForEach(group.items, id: \.index) { source in
                                        Button(source.name ?? "Source \(source.index + 1)") {
                                            rememberedRows[.sources] = .source(sources.videoId, source.index)
                                            Task {
                                                switch source.kind {
                                                case .torrent:
                                                    if let request = await model.prepare(index: source.index, expectedRevision: state.revision) { selectedSource = SelectedSource(request: .torrent(request)) }
                                                case .http:
                                                    if let request = await model.prepareHTTPS(index: source.index, expectedRevision: state.revision) { selectedSource = SelectedSource(request: .https(request)) }
                                                case .unsupported: break
                                                }
                                            }
                                        }.focused($focusedRow, equals: .source(sources.videoId, source.index))
                                        .disabled(source.kind == .unsupported)
                                        Text("\(source.transportName) · \(source.sizeLabel)").font(.caption).foregroundStyle(.secondary)
                                        if let description = source.description, !description.isEmpty { Text(description).font(.caption) }
                                        if source.kind == .unsupported { Text("This source format is not supported yet.").font(.caption) }
                                    }
                                }
                                if sources.status == .ready, sources.items.isEmpty { Text("No sources found. Install a compatible Stremio add-on.") }
                                if sources.offset > 0 { Button("Previous sources") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showSources(offset: max(0, sources.offset - 24), expectedRevision: $1) } } } }
                                if sources.offset + sources.items.count < sources.total { Button("More sources") { Task { await model.perform(expectedRevision: state.revision) { try await $0.showSources(offset: sources.offset + sources.items.count, expectedRevision: $1) } } } }
                            }
                        }
                    }
                }
                if screen != .catalogs { Button("Back", action: goBack) }
            } else if model.error != nil {
                Button("Retry Stremio") { screen = .catalogs; restart = UUID() }
            } else { ProgressView("Starting Stremio…") }
        }.padding(60)
        .task(id: screen) {
            await Task.yield()
            focusedRow = rememberedRows[screen]
        }
        .onExitCommand(perform: backAction)
        .task(id: restart) { await model.run() }
        .fullScreenCover(isPresented: $showSignIn) {
            AccountSignInView(model: signIn, home: model,
                onClose: { showSignIn = false; screen = .catalogs; restart = UUID() },
                onSuccess: { showSignIn = false; screen = .catalogs; restart = UUID() })
        }
        .fullScreenCover(item: $selectedSource, onDismiss: { focusedRow = rememberedRows[.sources] }) { source in
            SourcePlaybackView(request: source.request,
                               acquireAccounting: { await model.accounting(for: source.request) },
                               discoverSubtitles: model.subtitleDiscovery(for: source.request))
        }
    }
}
