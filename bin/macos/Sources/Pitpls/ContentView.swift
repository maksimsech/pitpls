import SwiftUI
import PitCore

struct ContentView: View {
    @Environment(AppModel.self) private var model
    @State private var addingYear = false

    var body: some View {
        @Bindable var model = model
        NavigationSplitView {
            List(selection: $model.destination) {
                Section("Library") {
                    sidebarItem(.overview)
                    sidebarItem(.dividends)
                    sidebarItem(.interests)
                    sidebarItem(.crypto)
                }
                Section("Data") {
                    sidebarItem(.imports)
                    sidebarItem(.rates)
                }
            }
            .listStyle(.sidebar)
            .navigationSplitViewColumnWidth(min: 180, ideal: 210, max: 260)
            .safeAreaInset(edge: .bottom) {
                HStack {
                    Label("Pitpls", systemImage: "externaldrive")
                        .font(.caption).foregroundStyle(.secondary)
                    Spacer()
                    SettingsLink { Image(systemName: "gearshape") }
                        .buttonStyle(.plain).help("Settings (⌘,)")
                }
                .padding()
            }
        } detail: {
            Group {
                if model.core != nil {
                    switch model.destination ?? .overview {
                    case .overview: OverviewView()
                    case .dividends: RecordsView(kind: .dividend)
                    case .interests: RecordsView(kind: .interest)
                    case .crypto: RecordsView(kind: .crypto)
                    case .imports: ImportsView()
                    case .rates: RatesView()
                    }
                } else if model.isConnecting {
                    ProgressView("Opening your library…").frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    ContentUnavailableView {
                        Label("Library unavailable", systemImage: "externaldrive.badge.exclamationmark")
                    } description: {
                        Text("Pitpls couldn’t open its database.")
                    } actions: {
                        Button("Try Again") { Task { await model.connect() } }
                    }
                }
            }
            .background(.background)
            .navigationTitle((model.destination ?? .overview).title)
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    if model.destination != .imports && model.destination != .rates {
                        Picker("Year", selection: $model.year) {
                            Text("All years").tag(nil as Int32?)
                            ForEach(model.years, id: \.self) { year in
                                Text(String(year)).tag(Optional(year))
                            }
                        }
                        .labelsHidden().frame(width: 115)
                        .help("Filter records by year")
                    }
                }
                ToolbarItem(placement: .primaryAction) {
                    Menu {
                        Button("Add Year…") { addingYear = true }
                        Button("Refresh", systemImage: "arrow.clockwise") { model.refresh() }
                    } label: { Image(systemName: "ellipsis.circle") }
                    .help("Library actions")
                    .disabled(model.core == nil || model.isWorking)
                }
            }
            .safeAreaInset(edge: .bottom, spacing: 0) {
                if model.isWorking || model.notice != nil {
                    HStack(spacing: 8) {
                        if model.isWorking { ProgressView().controlSize(.small); Text("Updating library…") }
                        else if let notice = model.notice {
                            Image(systemName: "checkmark.circle.fill").foregroundStyle(.secondary)
                            Text(notice)
                            Spacer()
                            Button { model.notice = nil } label: { Image(systemName: "xmark") }
                                .buttonStyle(.plain).help("Dismiss")
                        }
                    }
                    .font(.callout).padding(10).frame(maxWidth: .infinity, alignment: .leading)
                    .background(.background)
                }
            }
        }
        .frame(minWidth: 860, minHeight: 560)
        .task { await model.connect() }
        .sheet(isPresented: $addingYear) { AddYearView() }
        .alert("Unable to Complete Action", isPresented: Binding(
            get: { model.error != nil }, set: { if !$0 { model.error = nil } }
        )) { Button("OK") { model.error = nil } } message: { Text(model.error ?? "") }
    }

    private func sidebarItem(_ destination: Destination) -> some View {
        Label(destination.title, systemImage: destination.symbol).tag(destination)
    }
}

struct AddYearView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    @State private var year = String(Calendar.current.component(.year, from: .now))
    @State private var error: String?
    @State private var saving = false

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text("Add Year").font(.title2.bold())
            TextField("Year", text: $year)
                .textFieldStyle(.roundedBorder)
            if let error { Text(error).foregroundStyle(.red).font(.callout) }
            HStack {
                Spacer()
                Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction)
                Button("Add") {
                    Task {
                        guard let value = Int32(year) else {
                            error = "Enter a year from 1900 to 2100."; return
                        }
                        saving = true
                        defer { saving = false }
                        do {
                            guard try await model.mutate(operation: { core in
                                try await core.addYear(year: value)
                            }) else { return }
                            model.year = value
                            dismiss()
                        } catch { self.error = errorText(error) }
                    }
                }
                .keyboardShortcut(.defaultAction).disabled(saving || model.isWorking)
            }
        }.padding(24).frame(width: 330).interactiveDismissDisabled(saving)
    }
}
