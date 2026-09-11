import SwiftUI
import UniformTypeIdentifiers
import PitCore

struct ImportsView: View {
    @Environment(AppModel.self) private var model
    @State private var source: ImportSource?
    @State private var choosingFile = false
    @State private var receipt: String?
    private let sources = importSources()

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Import your statements").font(.largeTitle.bold())
                    Text("Choose a provider to add financial records to your library.")
                        .foregroundStyle(.secondary)
                }
                GroupBox {
                    VStack(spacing: 0) {
                        ForEach(Array(sources.enumerated()), id: \.offset) { index, source in
                            if index > 0 { Divider().padding(.leading, 60) }
                            HStack(spacing: 16) {
                                Image(systemName: source.extensions.contains("pdf") ? "doc.richtext" : "tablecells")
                                    .font(.title2).foregroundStyle(.secondary)
                                    .frame(width: 40, height: 44)
                                VStack(alignment: .leading, spacing: 4) {
                                    Text(source.name).font(.headline)
                                    Text(source.outputs.joined(separator: " · "))
                                        .font(.callout).foregroundStyle(.secondary)
                                }
                                Spacer()
                                Text(source.extensions.joined(separator: ", ").uppercased())
                                    .font(.caption.monospaced()).foregroundStyle(.secondary)
                                Button("Choose File…") {
                                    self.source = source; choosingFile = true
                                }.disabled(model.isWorking)
                            }.padding(12)
                        }
                    }
                }
                if let receipt {
                    GroupBox {
                        Label(receipt, systemImage: "checkmark.circle.fill")
                            .foregroundStyle(.secondary).frame(maxWidth: .infinity, alignment: .leading).padding(10)
                    }
                }
                VStack(alignment: .leading, spacing: 10) {
                    Label("Stored on your Mac", systemImage: "internaldrive")
                        .font(.headline)
                    Text("Statements are read locally. Your imported records stay in one library on your Mac.")
                        .foregroundStyle(.secondary)
                    Button("Review Exchange Rates", systemImage: "chart.xyaxis.line") { model.destination = .rates }
                }.padding(8)
            }.padding(28).frame(maxWidth: 960, alignment: .leading).frame(maxWidth: .infinity)
        }
        .fileImporter(isPresented: $choosingFile,
            allowedContentTypes: (source?.extensions ?? ["csv"]).compactMap { UTType(filenameExtension: $0) },
            allowsMultipleSelection: false
        ) { result in
            switch result {
            case .success(let urls):
                guard let url = urls.first, let source else { return }
                Task {
                    let scoped = url.startAccessingSecurityScopedResource()
                    defer { if scoped { url.stopAccessingSecurityScopedResource() } }
                    receipt = nil
                    await model.perform("Import completed.") { core in
                        let result = try await core.runImport(kind: source.kind, file: url.path)
                        receipt = "\(url.lastPathComponent): \(result.dividends) dividends, \(result.interests) interests, \(result.cryptos) crypto records imported."
                    }
                }
            case .failure(let error): model.error = errorText(error)
            }
        }
    }
}
