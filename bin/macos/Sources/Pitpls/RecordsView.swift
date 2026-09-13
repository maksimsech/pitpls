import SwiftUI
import PitCore

enum RecordKind: String {
    case dividend, interest, crypto
    var title: String { self == .crypto ? "Crypto" : rawValue.capitalized + "s" }
    var singular: String { rawValue.capitalized }
    var symbol: String {
        switch self { case .dividend: "chart.bar.xaxis"; case .interest: "percent"; case .crypto: "bitcoinsign.circle" }
    }
}

struct RecordRow: Identifiable {
    let id: String
    let date: String
    let title: String
    let provider: String
    let amount: Amount
    let draft: RecordDraft
    let details: [RecordDetail]
    var sortValue: Foundation.Decimal {
        Foundation.Decimal(string: amount.value, locale: Locale(identifier: "en_US_POSIX")) ?? 0
    }
}

struct RecordDetail: Identifiable {
    let label: String
    let value: String
    var id: String { label }
    init(_ label: String, _ value: String) { self.label = label; self.value = value }
}

struct RecordsView: View {
    @Environment(AppModel.self) private var model
    let kind: RecordKind
    @State private var rows: [RecordRow] = []
    @State private var metrics: [Metric] = []
    @State private var selection: Set<String> = []
    @State private var sortOrder = [KeyPathComparator(\RecordRow.date, order: .reverse)]
    @State private var search = ""
    @State private var draft: RecordDraft?
    @State private var deleteIDs: Set<String> = []
    @State private var showDelete = false
    @State private var showInspector = false
    @State private var loading = true
    @State private var error: String?

    private var visibleRows: [RecordRow] {
        rows.filter { search.isEmpty || "\($0.title) \($0.provider) \($0.date) \($0.id)"
            .localizedStandardContains(search) }.sorted(using: sortOrder)
    }
    private var selectedRow: RecordRow? {
        selection.count == 1 ? rows.first { selection.contains($0.id) } : nil
    }

    var body: some View {
        VStack(spacing: 0) {
            if !metrics.isEmpty { MetricStrip(items: metrics).padding(20) }
            if loading {
                ProgressView("Loading \(kind.title.lowercased())…").frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let error {
                LoadError(message: error) { model.refresh() }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if rows.isEmpty {
                ContentUnavailableView {
                    Label("No \(kind.title.lowercased()) yet", systemImage: kind.symbol)
                } description: {
                    Text("Import a statement or add your first record for this period.")
                } actions: {
                    Button("Import Records…") { model.destination = .imports }
                    Button("Add \(kind.singular)") { draft = RecordDraft(kind: kind, year: model.year) }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                Table(visibleRows, selection: $selection, sortOrder: $sortOrder) {
                    TableColumn("Date", value: \.date).width(min: 90, ideal: 100)
                    TableColumn(kind == .crypto ? "Action" : "Record", value: \.title)
                    TableColumn("Provider", value: \.provider)
                    TableColumn("Amount", value: \.sortValue) { row in
                        Text(money(row.amount.value, currency: currencyCode(row.amount.currency)))
                            .monospacedDigit().frame(maxWidth: .infinity, alignment: .trailing)
                    }.alignment(.trailing)
                }
                .scrollContentBackground(.hidden)
                .background(.background)
                .contextMenu(forSelectionType: String.self) { ids in
                    if ids.count == 1, let row = rows.first(where: { ids.contains($0.id) }) {
                        Button("Edit…") { draft = row.draft }
                        Button("Show Details") { selection = ids; showInspector = true }
                    }
                    if !ids.isEmpty {
                        Button("Delete…", role: .destructive) { deleteIDs = ids; showDelete = true }
                    }
                } primaryAction: { ids in
                    if let row = rows.first(where: { ids.contains($0.id) }) { draft = row.draft }
                }
                .onDeleteCommand {
                    if !selection.isEmpty, !model.isWorking { deleteIDs = selection; showDelete = true }
                }
                .disabled(model.isWorking)
                HStack {
                    Text("\(visibleRows.count) records")
                    if !selection.isEmpty { Text("· \(selection.count) selected") }
                    Spacer()
                    Text("Amounts in original currency")
                }.font(.caption).foregroundStyle(.secondary).padding(10).background(.background)
            }
        }
        .searchable(text: $search, prompt: "Search \(kind.title.lowercased())")
        .toolbar {
            ToolbarItemGroup {
                Button { draft = RecordDraft(kind: kind, year: model.year) } label: {
                    Label("Add \(kind.singular)", systemImage: "plus")
                }.keyboardShortcut("n").disabled(model.isWorking)
                Button { if let selectedRow { draft = selectedRow.draft } } label: {
                    Label("Edit Record", systemImage: "square.and.pencil")
                }.disabled(selectedRow == nil || model.isWorking)
                Button { deleteIDs = selection; showDelete = true } label: {
                    Label("Delete Records", systemImage: "trash")
                }.disabled(selection.isEmpty || model.isWorking)
                Button { showInspector.toggle() } label: {
                    Label("Record Details", systemImage: "sidebar.right")
                }
            }
        }
        .inspector(isPresented: $showInspector) {
            if let row = selectedRow {
                Form {
                    Section("Record") {
                        LabeledContent("ID", value: row.id)
                        LabeledContent("Date", value: row.date)
                        LabeledContent("Provider", value: row.provider)
                    }
                    Section("Calculation") {
                        ForEach(row.details) { detail in LabeledContent(detail.label, value: detail.value) }
                    }
                    Button("Edit Record…") { draft = row.draft }.disabled(model.isWorking)
                }.formStyle(.grouped).textSelection(.enabled)
            } else {
                ContentUnavailableView("Select a Record", systemImage: "doc.text.magnifyingglass",
                    description: Text("Select one row to see its calculation details."))
            }
        }
        .sheet(item: $draft) { RecordEditor(draft: $0) }
        .confirmationDialog("Delete \(deleteIDs.count) records?", isPresented: $showDelete) {
            Button("Delete Records", role: .destructive) {
                let ids = Array(deleteIDs)
                Task {
                    await model.perform("Records deleted.") { core in
                        switch kind {
                        case .dividend: _ = try await core.deleteDividends(ids: ids)
                        case .interest: _ = try await core.deleteInterests(ids: ids)
                        case .crypto: _ = try await core.deleteCryptos(ids: ids)
                        }
                    }
                }
            }
        } message: { Text("This removes the selected records from your library and cannot be undone.") }
        .task(id: "\(kind.rawValue)-\(model.loadID)") { await load() }
    }

    private func load() async {
        guard let core = model.core else { return }
        loading = true; error = nil; rows = []; metrics = []; selection = []
        do {
            let newRows: [RecordRow]
            let newMetrics: [Metric]
            switch kind {
            case .dividend:
                let data = try await core.loadDividends(year: model.year)
                newMetrics = [Metric(title: "Income", value: data.income, reference: "I-65"),
                              Metric(title: "Tax to pay", value: data.toPay, reference: "G-47"),
                              Metric(title: "Tax paid", value: data.paid, reference: "G-48")]
                newRows = data.calculated.map { value in
                    RecordRow(id: value.id, date: value.date, title: value.ticker, provider: value.provider,
                        amount: value.value, draft: RecordDraft(value), details: [
                            RecordDetail("Country", value.country),
                            RecordDetail("NBP date", value.nbpDate),
                            RecordDetail("Value in PLN", money(value.calculatedValue)),
                            RecordDetail("Tax to pay", money(value.calculatedToPay)),
                            RecordDetail("Tax paid", money(value.taxPaid.value, currency: currencyCode(value.taxPaid.currency))),
                            RecordDetail("Tax paid in PLN", money(value.calculatedTaxPaid)),
                            RecordDetail("Maximum credit", money(value.maxTaxPaid)),
                            RecordDetail("Credit used", money(value.usedTaxPaid)),
                        ])
                }
            case .interest:
                let data = try await core.loadInterests(year: model.year)
                newMetrics = [Metric(title: "Income", value: data.income, reference: "I-65"),
                              Metric(title: "Tax to pay", value: data.toPay, reference: "G-47")]
                newRows = data.calculated.map { value in
                    RecordRow(id: value.id, date: value.date, title: "Interest", provider: value.provider,
                        amount: value.value, draft: RecordDraft(value), details: [
                            RecordDetail("NBP date", value.nbpDate),
                            RecordDetail("Value in PLN", money(value.calculatedValue)),
                            RecordDetail("Tax to pay", money(value.toPay)),
                        ])
                }
            case .crypto:
                let data = try await core.loadCryptos(year: model.year)
                newMetrics = [Metric(title: "Income", value: data.income, reference: "E-36"),
                              Metric(title: "Costs", value: data.costs, reference: "E-37")]
                newRows = data.calculated.map { value in
                    RecordRow(id: value.id, date: value.date, title: value.action == .fiatBuy ? "Buy" : "Sell",
                        provider: value.provider, amount: value.value, draft: RecordDraft(value), details: [
                            RecordDetail("NBP date", value.nbpDate),
                            RecordDetail("Value in PLN", money(value.calculatedValue)),
                            RecordDetail("Fee", money(value.fee.value, currency: currencyCode(value.fee.currency))),
                            RecordDetail("Fee in PLN", money(value.calculatedFee)),
                        ])
                }
            }
            guard !Task.isCancelled else { return }
            rows = newRows; metrics = newMetrics
        } catch { if !Task.isCancelled { self.error = errorText(error) } }
        if !Task.isCancelled { loading = false }
    }
}
