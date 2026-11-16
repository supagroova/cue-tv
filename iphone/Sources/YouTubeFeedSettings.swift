import SwiftUI

struct YouTubeFeedSettings: View {
    @ObservedObject var model: RemoteModel
    @Environment(\.dismiss) private var dismiss
    var body: some View {
        NavigationStack {
            Form {
                Section {
                    ForEach(model.youtubeFeeds.profiles) { feed in
                        NavigationLink {
                            YouTubeFeedEditor(model: model, id: feed.id, initialName: feed.name)
                        } label: {
                            HStack {
                                Text(feed.name)
                                Spacer()
                                if feed.id == model.youtubeFeeds.selectedID { Image(systemName: "checkmark").accessibilityLabel("Selected") }
                            }.frame(minHeight: 44)
                        }
                    }
                } header: { Text("Feeds on this iPhone") } footer: {
                    Text("Each feed keeps its own YouTube sign-in. Adding a feed here does not add it to another phone.")
                }
                Section {
                    NavigationLink {
                        YouTubeFeedEditor(model: model, id: nil, initialName: "")
                    } label: { Label("Add feed", systemImage: "plus") }
                    .disabled(model.youtubeFeeds.profiles.count >= YouTubeFeeds.maximum)
                }
            }
            .navigationTitle("YouTube feeds").navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        }
    }
}

private struct YouTubeFeedEditor: View {
    @ObservedObject var model: RemoteModel
    let id: UUID?
    @State private var name: String
    @State private var confirmingRemoval = false
    @Environment(\.dismiss) private var dismiss
    init(model: RemoteModel, id: UUID?, initialName: String) {
        self.model = model; self.id = id; _name = State(initialValue: initialName)
    }
    var body: some View {
        Form {
            Section {
                TextField("Feed name", text: $name).textInputAutocapitalization(.words).autocorrectionDisabled()
            } footer: { Text("Use a different name for each feed, such as Rupert or Ursula.") }
            if let id {
                Section {
                    Button("Use this feed") { model.selectYouTubeFeed(id); model.managingYouTubeFeeds = false }
                    Button("Sign in or change account") {
                        model.selectYouTubeFeed(id); model.surface?.feed.account(); model.managingYouTubeFeeds = false
                    }
                }
                if model.youtubeFeeds.profiles.count > 1 {
                    Section { Button("Remove feed", role: .destructive) { confirmingRemoval = true } }
                }
            } else {
                Section { Text("After adding this feed, sign in to its account in YouTube.").font(.footnote) }
            }
        }
        .navigationTitle(id == nil ? "Add YouTube feed" : "Edit feed").navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .confirmationAction) {
                Button(id == nil ? "Add" : "Save") {
                    if let id { model.renameYouTubeFeed(id, to: name); dismiss() }
                    else if model.addYouTubeFeed(name) { model.surface?.feed.account(); model.managingYouTubeFeeds = false }
                }.disabled(!model.youtubeFeeds.canUseName(name, except: id))
            }
        }
        .confirmationDialog("Remove \(name)?", isPresented: $confirmingRemoval, titleVisibility: .visible) {
            if let id { Button("Remove feed", role: .destructive) { model.removeYouTubeFeed(id); dismiss() } }
            Button("Cancel", role: .cancel) {}
        } message: { Text("Removes this feed and its website sign-in from this iPhone.") }
    }
}
