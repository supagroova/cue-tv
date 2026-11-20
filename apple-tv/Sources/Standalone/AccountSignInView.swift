import SwiftUI

struct AccountSignInView: View {
    @Bindable var model: AccountSignInModel
    let home: StandaloneModel
    let onClose: @MainActor () -> Void
    let onSuccess: @MainActor () -> Void
    @State private var email = ""
    @State private var password = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 28) {
            Text("Sign in to Stremio").font(.largeTitle)
            TextField("Email", text: $email).textContentType(.username).autocorrectionDisabled()
                .disabled(model.busy)
            SecureField("Password", text: $password).textContentType(.password)
                .disabled(model.busy)
            if model.busy { ProgressView("Signing in…") }
            if let error = model.error { Text(error).foregroundStyle(.orange) }
            Button("Sign in") {
                model.submit(email: email.trimmingCharacters(in: .whitespacesAndNewlines), password: password,
                             home: home, onSuccess: onSuccess)
                password = ""
            }.disabled(model.busy || email.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || password.isEmpty)
            Button("Cancel") { close() }
        }
        .frame(maxWidth: 1000).padding(60)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color.black)
        .onExitCommand { close() }
        .onDisappear { password = ""; Task { await model.cancel() } }
    }
    private func close() {
        password = ""
        Task { if await model.cancel() { onClose() } }
    }
}
