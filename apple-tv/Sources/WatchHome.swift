import SwiftUI

enum WatchPage: String, CaseIterable, Identifiable {
    case home = "Watch Now", search = "Search", series = "TV Shows", movies = "Movies", library = "Library"
    var id: String { rawValue }
    var symbol: String {
        switch self { case .home: "play.rectangle.fill"; case .search: "magnifyingglass"; case .series: "tv"; case .movies: "film"; case .library: "rectangle.stack" }
    }
}

struct WatchHome: View {
    @Bindable var model: WatchModel
    @State private var page: WatchPage? = .home
    var body: some View {
        NavigationSplitView {
            List(selection: $page) {
                Section {
                    ForEach(WatchPage.allCases) { page in Label(page.rawValue, systemImage: page.symbol).tag(page) }
                } header: { Text("Cue TV").font(.system(size: 42, weight: .bold)).padding(.bottom, 24) }
                Section("Apps") {
                    Button("YouTube", systemImage: "play.rectangle", action: model.youtube)
                }
            }
            .navigationTitle("")
            .navigationSplitViewColumnWidth(300)
        } detail: {
            Group {
                if page == .search { SearchView(model: model) }
                else { browse }
            }
            .background(Color(white: 0.035))
        }
        .fullScreenCover(item: $model.selected) { item in TitleDetail(item: item, model: model) }
        .onChange(of: model.homeRequest) { _, _ in page = .home }
    }

    @ViewBuilder private var browse: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 44) {
                if page == .home, let hero = model.catalogue.series.first ?? model.catalogue.movies.first {
                    Hero(item: hero, title: "Watch Now") { model.selected = hero }
                } else { Text((page ?? .home).rawValue).font(.system(size: 54, weight: .bold)).padding(.top, 44).padding(.horizontal, 64) }
                if let error = model.error {
                    HStack(spacing: 30) {
                        Text(error).font(.body).foregroundStyle(.secondary)
                        Button("Try Again") { Task { await model.refresh() } }
                    }.padding(.horizontal, 64)
                }
                if model.loading && model.catalogue.series.isEmpty { ProgressView("Loading your TV").frame(maxWidth: .infinity).padding(100) }
                if page == .home || page == .library {
                    shelf("Continue Watching", model.library.filter { $0.inProgress == true }, landscape: true)
                    shelf("Recently Watched", model.library.filter { $0.inProgress != true })
                    if page == .library && model.library.isEmpty && !model.loading {
                        ContentUnavailableView(model.linked ? "Your library is ready" : "Connect Stremio", systemImage: "rectangle.stack",
                            description: Text(model.linked ? "The shows you watch and save in Stremio will appear here." : "Connect Stremio from Watch on your iPhone, then sign in to the same account in Stremio on Apple TV."))
                    }
                }
                if page == .home || page == .series {
                    shelf("Trending Series", model.catalogue.trendingSeries ?? [])
                    shelf("Popular Series", model.catalogue.series)
                }
                if page == .home || page == .series || page == .movies {
                    ForEach(model.collections.filter { page == .home || $0.type == (page == .series ? "series" : "movie") }) { row in
                        shelf(row.title, row.items)
                    }
                }
                if page == .home || page == .movies {
                    shelf("Trending Films", model.catalogue.trendingMovies ?? [])
                    shelf("Popular Films", model.catalogue.movies)
                }
                if !model.loading && model.catalogue.series.isEmpty && model.catalogue.movies.isEmpty && page != .library {
                    ContentUnavailableView("Couldn't load your TV", systemImage: "wifi.exclamationmark", description: Text("Check Sitting Room TV is switched on, then try again."))
                    Button("Try Again") { Task { await model.refresh() } }.padding(.horizontal, 64)
                }
            }.padding(.bottom, 100)
        }.ignoresSafeArea(.container, edges: [.top, .horizontal])
    }
    @ViewBuilder private func shelf(_ label: String, _ items: [Title], landscape: Bool = false) -> some View {
        if !items.isEmpty { TitleShelf(label: label, items: items, landscape: landscape) { model.selected = $0 } }
    }
}

struct Hero: View {
    let item: Title
    let title: String
    let action: () -> Void
    var body: some View {
        ZStack(alignment: .bottomLeading) {
            GeometryReader { geo in
                AsyncImage(url: item.backgroundURL ?? item.posterURL) { image in
                    image.resizable().scaledToFill().frame(width: geo.size.width, height: geo.size.height).clipped()
                } placeholder: { Color(white: 0.10) }
            }
            LinearGradient(stops: [.init(color: .black.opacity(0.25), location: 0), .init(color: .black.opacity(0.12), location: 0.3), .init(color: Color(white: 0.035), location: 1)], startPoint: .top, endPoint: .bottom)
            LinearGradient(colors: [.black.opacity(0.6), .clear], startPoint: .leading, endPoint: .trailing)
            VStack(alignment: .leading, spacing: 18) {
                Text(title.uppercased()).font(.system(size: 20, weight: .semibold)).tracking(2).foregroundStyle(.white.opacity(0.65))
                Text(item.title).font(.system(size: 64, weight: .bold)).lineLimit(2)
                Text([item.year, item.type == "series" ? "Series" : "Film"].compactMap { $0 }.joined(separator: "  ·  "))
                    .font(.system(size: 22, weight: .medium)).foregroundStyle(.white.opacity(0.7))
                if let description = item.description, !description.isEmpty { Text(description).font(.system(size: 23)).lineLimit(3).foregroundStyle(.white.opacity(0.85)) }
                Button("View Details", systemImage: "info.circle", action: action).padding(.top, 8)
            }.frame(maxWidth: 680, alignment: .leading).padding(.horizontal, 64).padding(.bottom, 35)
        }.frame(height: 600).clipped()
    }
}

struct TitleShelf: View {
    let label: String
    let items: [Title]
    var landscape = false
    let action: (Title) -> Void
    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text(label).font(.system(size: 30, weight: .bold)).padding(.horizontal, 64)
            ScrollView(.horizontal) {
                LazyHStack(alignment: .top, spacing: 30) {
                    ForEach(items, id: \.identity) { item in
                        Button { action(item) } label: {
                            VStack(alignment: .leading, spacing: 12) {
                                AsyncImage(url: landscape ? (item.backgroundURL ?? item.posterURL) : item.posterURL) { image in image.resizable().scaledToFill() }
                                    placeholder: { ZStack { Color(white: 0.14); Image(systemName: "play.rectangle").font(.largeTitle).foregroundStyle(.secondary) } }
                                    .frame(width: landscape ? 330 : 190, height: landscape ? 186 : 285).clipped().clipShape(RoundedRectangle(cornerRadius: 12))
                                if let progress = item.progress, progress > 0 { ProgressView(value: min(progress, 1)).tint(.white) }
                                Text(item.title).font(.system(size: 20, weight: .medium)).lineLimit(2).frame(height: 50, alignment: .topLeading)
                            }.frame(width: landscape ? 330 : 190)
                        }.buttonStyle(.card).accessibilityLabel(item.title)
                    }
                }.padding(.horizontal, 64).padding(.vertical, 18)
            }.scrollClipDisabled()
        }.focusSection()
    }
}

private struct TitleDetail: View {
    let item: Title
    @Bindable var model: WatchModel
    @Environment(\.dismiss) private var dismiss
    @State private var choosing = false
    var body: some View {
        ZStack(alignment: .leading) {
            GeometryReader { geo in
                AsyncImage(url: item.backgroundURL ?? item.posterURL) { image in image.resizable().scaledToFill().frame(width: geo.size.width, height: geo.size.height).clipped() }
                    placeholder: { Color(white: 0.08) }
            }.ignoresSafeArea()
            LinearGradient(colors: [.black.opacity(0.9), .black.opacity(0.65), .black.opacity(0.1)], startPoint: .leading, endPoint: .trailing).ignoresSafeArea()
            VStack(alignment: .leading, spacing: 28) {
                Text(item.type == "series" ? "SERIES" : "FILM").font(.headline).tracking(2).foregroundStyle(.secondary)
                Text(item.title).font(.system(size: 72, weight: .bold)).lineLimit(3)
                if let year = item.year { Text(year).font(.title3).foregroundStyle(.secondary) }
                if let description = item.description { Text(description).font(.title3).lineLimit(6) }
                Button(item.type == "series" ? "Episodes & Sources" : "Choose a source", systemImage: "play.fill") { choosing = true }.padding(.top, 12)
                Button("Back", systemImage: "chevron.left") { dismiss() }
                if let error = model.error { Text(error).foregroundStyle(.orange).font(.callout) }
            }.frame(maxWidth: 820, alignment: .leading).padding(100)
        }.onExitCommand { dismiss() }
            .fullScreenCover(isPresented: $choosing) { CueEpisodeBrowser(item: item, service: model.service) }
    }
}

private struct SearchView: View {
    @Bindable var model: WatchModel
    @State private var query = ""
    @State private var results: [Title] = []
    @State private var searching = false
    @State private var failed = false
    @State private var retry = 0
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 36) {
                Text("Search").font(.system(size: 54, weight: .bold)).padding(.top, 44)
                TextField("Films and TV shows", text: $query).autocorrectionDisabled()
                if searching { ProgressView("Searching").padding(.vertical, 70) }
                else if failed {
                    Text("Couldn't search right now. Check Sitting Room TV is switched on.").foregroundStyle(.secondary)
                    Button("Try Again") { retry += 1 }
                } else if query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    Text("Find something to watch on Stremio.").font(.title3).foregroundStyle(.secondary)
                } else if results.isEmpty {
                    Text("No titles found for “\(query)”.").font(.title3).foregroundStyle(.secondary)
                }
            }.padding(.horizontal, 64)
            if !searching && !failed {
                if results.contains(where: { $0.type == "series" }) { TitleShelf(label: "TV Shows", items: results.filter { $0.type == "series" }) { model.selected = $0 } }
                if results.contains(where: { $0.type == "movie" }) { TitleShelf(label: "Films", items: results.filter { $0.type == "movie" }) { model.selected = $0 } }
            }
        }
        .task(id: "\(query)|\(retry)") {
            let term = query.trimmingCharacters(in: .whitespacesAndNewlines)
            results = []; failed = false; searching = !term.isEmpty
            guard !term.isEmpty else { return }
            do {
                try await Task.sleep(for: .milliseconds(300))
                let response = try await model.service.request("api/stremio/search", as: SearchResults.self, query: String(term.prefix(100)))
                try Task.checkCancellation()
                results = response.items; searching = false
            } catch {
                guard !Task.isCancelled else { return }
                failed = true; searching = false
            }
        }
    }
}
