import SwiftUI
import UniformTypeIdentifiers
import PitCore

private struct ExchangeRateRow: Identifiable {
    let date: String
    let currency: String
    let rate: String
    var id: String { "\(date)-\(currency)" }
}

struct RatesView: View {
    @Environment(AppModel.self) private var model
    @State private var rows: [ExchangeRateRow] = []
    @State private var currencies: [String] = []
    @State private var currency = "All"
    @State private var search = ""
    @State private var selection: Set<String> = []
    @State private var sortOrder = [KeyPathComparator(\ExchangeRateRow.date, order: .reverse)]
    @State private var year = Int32(Calendar.current.component(.year, from: .now))
    @State private var choosingFile = false
    @State private var resetting = false
    @State private var loading = true
    @State private var error: String?

    private var visibleRows: [ExchangeRateRow] {
        rows.filter { (currency == "All" || $0.currency == currency)
            && (search.isEmpty || "\($0.date) \($0.currency)".localizedStandardContains(search)) }
            .sorted(using: sortOrder)
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 12) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("NBP exchange rates").font(.headline)
                    Text("PLN per unit of foreign currency").font(.callout).foregroundStyle(.secondary)
                }
                Spacer()
                Picker("Year to download", selection: $year) {
                    ForEach(Array((1900...2100).reversed()), id: \.self) { value in Text(String(value)).tag(Int32(value)) }
                }.labelsHidden().frame(width: 95)
                Button("Download from NBP") {
                    Task {
                        await model.perform("NBP rates downloaded.") { core in _ = try await core.importApi(year: year) }
                    }
                }.disabled(model.isWorking)
            }.padding(20)
            Divider()
            if loading {
                ProgressView("Loading rates…").frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let error {
                LoadError(message: error) { model.refresh() }.frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if rows.isEmpty {
                ContentUnavailableView("No Exchange Rates", systemImage: "chart.xyaxis.line",
                    description: Text("Download a year from NBP or import a rates CSV to calculate your records."))
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                Table(visibleRows, selection: $selection, sortOrder: $sortOrder) {
                    TableColumn("Date", value: \.date)
                    TableColumn("Currency", value: \.currency)
                    TableColumn("Rate in PLN") { row in Text(row.rate).monospacedDigit() }
                }
                .scrollContentBackground(.hidden)
                .background(.background)
                HStack {
                    Text("\(visibleRows.count) rates")
                    Spacer()
                    Text("Source: Narodowy Bank Polski")
                }.font(.caption).foregroundStyle(.secondary).padding(10).background(.background)
            }
        }
        .searchable(text: $search, prompt: "Search dates or currencies")
        .toolbar {
            ToolbarItemGroup {
                Picker("Currency", selection: $currency) {
                    Text("All currencies").tag("All")
                    ForEach(currencies, id: \.self) { Text($0).tag($0) }
                }.labelsHidden().frame(width: 130)
                Button { choosingFile = true } label: { Label("Import CSV", systemImage: "square.and.arrow.down") }
                    .disabled(model.isWorking)
                Button { resetting = true } label: { Label("Remove All Rates", systemImage: "trash") }
                    .disabled(rows.isEmpty || model.isWorking)
            }
        }
        .confirmationDialog("Remove all exchange rates?", isPresented: $resetting) {
            Button("Remove All Rates", role: .destructive) {
                Task { await model.perform("Exchange rates removed.") { core in _ = try await core.resetRates() } }
            }
        } message: { Text("Calculations that need foreign exchange rates will be unavailable until rates are loaded again.") }
        .fileImporter(isPresented: $choosingFile, allowedContentTypes: [.commaSeparatedText], allowsMultipleSelection: false) { result in
            switch result {
            case .success(let urls):
                guard let url = urls.first else { return }
                Task {
                    let scoped = url.startAccessingSecurityScopedResource()
                    defer { if scoped { url.stopAccessingSecurityScopedResource() } }
                    await model.perform("Exchange rates imported.") { core in _ = try await core.importCsv(file: url.path) }
                }
            case .failure(let error): model.error = errorText(error)
            }
        }
        .task(id: model.revision) {
            guard let core = model.core else { return }
            loading = true; error = nil; rows = []; selection = []
            do {
                let data = try await core.listRates()
                guard !Task.isCancelled else { return }
                currencies = data.currencies.map(currencyCode)
                if currency != "All", !currencies.contains(currency) { currency = "All" }
                rows = data.rows.flatMap { day in day.rates.map {
                    ExchangeRateRow(date: day.date, currency: currencyCode($0.currency), rate: $0.rate)
                } }
            } catch { if !Task.isCancelled { self.error = errorText(error) } }
            if !Task.isCancelled { loading = false }
        }
    }
}
