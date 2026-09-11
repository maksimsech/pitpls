import SwiftUI
import PitCore

struct OverviewView: View {
    @Environment(AppModel.self) private var model
    @State private var summary: TaxSummary?
    @State private var warnings: Warnings?
    @State private var error: String?
    @State private var loading = true

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Your tax overview").font(.largeTitle.bold())
                    Text(model.year.map { "Financial records for \($0)" } ?? "Financial records across all years")
                        .foregroundStyle(.secondary)
                }
                if let warnings, warnings.ratesEmpty && warnings.hasRecordsInYear {
                    GroupBox {
                        HStack(spacing: 12) {
                            Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.orange)
                            VStack(alignment: .leading, spacing: 4) {
                                Text("Exchange rates needed").fontWeight(.semibold)
                                Text("Load NBP rates to calculate your records in PLN.").foregroundStyle(.secondary)
                            }
                            Spacer()
                            Button("Open Rates") { model.destination = .rates }
                        }.padding(8)
                    }
                }
                if loading { ProgressView("Calculating overview…").frame(maxWidth: .infinity).padding(40) }
                else if let summary {
                    summarySection("Foreign income", symbol: "globe.europe.africa", items: [
                        Metric(title: "Income", value: summary.foreign.income, reference: "I-65"),
                        Metric(title: "Tax to pay", value: summary.foreign.taxToPay, reference: "G-47"),
                        Metric(title: "Tax paid", value: summary.foreign.taxPaid, reference: "G-48"),
                    ])
                    summarySection("Cryptocurrency", symbol: "bitcoinsign.circle", items: [
                        Metric(title: "Income", value: summary.crypto.income, reference: "E-36"),
                        Metric(title: "Costs", value: summary.crypto.costs, reference: "E-37"),
                    ])
                }
                if let error { LoadError(message: error) { model.refresh() } }
                GroupBox {
                    HStack(spacing: 16) {
                        Image(systemName: "tray.and.arrow.down").font(.title).foregroundStyle(.secondary)
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Bring your records together").font(.headline)
                            Text("Import statements from Trading 212, Revolut, or Coinbase.")
                                .foregroundStyle(.secondary)
                        }
                        Spacer()
                        Button("Import Records…") { model.destination = .imports }
                    }.padding(10)
                }
                Text("For informational use. Verify the data and calculations before using them.")
                    .font(.caption).foregroundStyle(.secondary)
            }.padding(28).frame(maxWidth: 1100, alignment: .leading).frame(maxWidth: .infinity)
        }
        .task(id: model.loadID) {
            guard let core = model.core else { return }
            loading = true; error = nil; summary = nil; warnings = nil
            do {
                let loadedWarnings = try await core.getWarnings(year: model.year)
                guard !Task.isCancelled else { return }
                warnings = loadedWarnings
                let loadedSummary = try await core.loadTaxSummary(year: model.year)
                guard !Task.isCancelled else { return }
                summary = loadedSummary
            } catch { if !Task.isCancelled { self.error = errorText(error) } }
            if !Task.isCancelled { loading = false }
        }
    }

    private func summarySection(_ title: String, symbol: String, items: [Metric]) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(title, systemImage: symbol).font(.headline)
            MetricStrip(items: items)
        }
    }
}

struct Metric: Identifiable {
    let title: String
    let value: String
    let reference: String
    var id: String { title }
}

struct MetricStrip: View {
    let items: [Metric]
    var body: some View {
        HStack(spacing: 12) {
            ForEach(items) { item in
                GroupBox {
                    VStack(alignment: .leading, spacing: 10) {
                        HStack {
                            Text(item.title).foregroundStyle(.secondary)
                            Spacer()
                            Text(item.reference).font(.caption).foregroundStyle(.tertiary)
                        }
                        Text(money(item.value)).font(.title2.weight(.semibold)).monospacedDigit()
                            .textSelection(.enabled).lineLimit(1).minimumScaleFactor(0.6)
                    }.padding(8).frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
    }
}

struct LoadError: View {
    let message: String
    let retry: () -> Void
    var body: some View {
        ContentUnavailableView {
            Label("Data unavailable", systemImage: "exclamationmark.triangle")
        } description: { Text(message).textSelection(.enabled) } actions: {
            Button("Try Again", action: retry)
        }
    }
}

// Formatting only: amounts remain exact decimal strings across the Rust boundary.
func money(_ value: String, currency: String = "PLN") -> String {
    guard let decimal = Foundation.Decimal(string: value, locale: Locale(identifier: "en_US_POSIX")) else {
        return "\(value) \(currency)"
    }
    return decimal.formatted(.currency(code: currency))
}

func currencyCode(_ currency: Currency) -> String { String(describing: currency).uppercased() }
