import SwiftUI
import PitCore

struct SettingsView: View {
    @Environment(AppModel.self) private var model
    @State private var rounding: DividendRounding = .sumToGroszy
    @State private var savedRounding: DividendRounding?
    @State private var error: String?
    @State private var saving = false
    @State private var removingYear: Int32?
    @State private var confirmRemoval = false

    var body: some View {
        Form {
            Section("Calculations") {
                Picker("Dividend rounding", selection: $rounding) {
                    Text("Sum to groszy").tag(DividendRounding.sumToGroszy)
                    Text("Sum to pay to złote").tag(DividendRounding.sumToPayToZlote)
                    Text("Sum both to złote").tag(DividendRounding.sumBothToZlote)
                    Text("All to złote").tag(DividendRounding.allToZlote)
                }.disabled(savedRounding == nil || saving)
                HStack {
                    if let error { Text(error).foregroundStyle(.red).font(.callout) }
                    Spacer()
                    if saving { ProgressView().controlSize(.small) }
                    Button("Save") {
                        Task {
                            saving = true; error = nil
                            defer { saving = false }
                            do {
                                guard try await model.mutate(operation: { core in
                                    try await core.updateSettings(settings: PitCore.Settings(dividendRounding: rounding))
                                }) else { return }
                                savedRounding = rounding
                            } catch { self.error = errorText(error) }
                        }
                    }.disabled(savedRounding == nil || rounding == savedRounding || saving || model.isWorking)
                }
            }
            Section("Years") {
                if model.years.isEmpty { Text("Add years using the library toolbar.").foregroundStyle(.secondary) }
                ForEach(model.years, id: \.self) { year in
                    LabeledContent(String(year)) {
                        Button(role: .destructive) { removingYear = year; confirmRemoval = true } label: {
                            Image(systemName: "minus.circle")
                        }.buttonStyle(.borderless).help("Remove \(year) from the year picker")
                            .disabled(model.isWorking)
                    }
                }
            }
            Section("Library") {
                LabeledContent("Storage", value: "On this Mac")
                Text(model.databaseURL.path).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)
                Button("Show in Finder") { NSWorkspace.shared.activateFileViewerSelecting([model.databaseURL]) }
            }
        }
        .formStyle(.grouped).frame(width: 510, height: 530)
        .task(id: model.core != nil) {
            await model.connect()
            guard let core = model.core else { return }
            do {
                let settings = try await core.loadSettings()
                guard !Task.isCancelled else { return }
                rounding = settings.dividendRounding; savedRounding = rounding; error = nil
            } catch { if !Task.isCancelled { self.error = errorText(error) } }
        }
        .confirmationDialog("Remove this year from the picker?", isPresented: $confirmRemoval) {
            Button("Remove Year", role: .destructive) {
                guard let year = removingYear else { return }
                Task {
                    do {
                        guard try await model.mutate(operation: { core in
                            _ = try await core.deleteYear(year: year)
                        }) else { return }
                        if model.year == year { model.year = nil }
                    } catch { self.error = errorText(error) }
                }
            }
        } message: { Text("Financial records are kept and remain visible under All years.") }
    }
}
