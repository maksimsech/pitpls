import SwiftUI
import PitCore

struct RecordDraft: Identifiable {
    let id = UUID()
    let kind: RecordKind
    var existingID: String?
    var customID = ""
    var date = Date.now
    var provider = ""
    var value = ""
    var currency: Currency = .usd
    var ticker = ""
    var country = "US"
    var secondary = "0"
    var secondaryCurrency: Currency = .usd
    var action: Action = .fiatBuy

    @MainActor init(kind: RecordKind, year: Int32?) {
        self.kind = kind
        if let year, year != Calendar.current.component(.year, from: date) {
            date = RecordDate.formatter.date(from: "\(year)-01-01") ?? date
        }
    }
    @MainActor init(_ record: CalculatedDividend) {
        kind = .dividend; existingID = record.id; customID = record.id
        date = RecordDate.formatter.date(from: record.date) ?? .now
        provider = record.provider; value = record.value.value; currency = record.value.currency
        ticker = record.ticker; country = record.country
        secondary = record.taxPaid.value; secondaryCurrency = record.taxPaid.currency
    }
    @MainActor init(_ record: CalculatedInterest) {
        kind = .interest; existingID = record.id; customID = record.id
        date = RecordDate.formatter.date(from: record.date) ?? .now
        provider = record.provider; value = record.value.value; currency = record.value.currency
    }
    @MainActor init(_ record: CalculatedCrypto) {
        kind = .crypto; existingID = record.id; customID = record.id
        date = RecordDate.formatter.date(from: record.date) ?? .now
        provider = record.provider; value = record.value.value; currency = record.value.currency
        secondary = record.fee.value; secondaryCurrency = record.fee.currency; action = record.action
    }

    @MainActor func save(using core: RustCore) async throws {
        let date = RecordDate.formatter.string(from: date)
        let id = customID.trimmingCharacters(in: .whitespacesAndNewlines)
        // Text is passed to the Rust use cases for decimal/date validation.
        switch kind {
        case .dividend:
            if let existingID {
                try await core.updateDividend(input: UpdateDividendInput(id: existingID, date: date,
                    ticker: ticker, value: value, valueCurrency: currency, taxPaid: secondary,
                    taxPaidCurrency: secondaryCurrency, country: country, provider: provider))
            } else {
                _ = try await core.createDividend(input: CreateDividendInput(id: id.isEmpty ? nil : id,
                    date: date, ticker: ticker, value: value, valueCurrency: currency, taxPaid: secondary,
                    taxPaidCurrency: secondaryCurrency, country: country, provider: provider))
            }
        case .interest:
            if let existingID {
                try await core.updateInterest(input: UpdateInterestInput(id: existingID, date: date,
                    value: value, valueCurrency: currency, provider: provider))
            } else {
                _ = try await core.createInterest(input: CreateInterestInput(id: id.isEmpty ? nil : id,
                    date: date, value: value, valueCurrency: currency, provider: provider))
            }
        case .crypto:
            if let existingID {
                try await core.updateCrypto(input: UpdateCryptoInput(id: existingID, date: date,
                    action: action, value: value, valueCurrency: currency, fee: secondary,
                    feeCurrency: secondaryCurrency, provider: provider))
            } else {
                _ = try await core.createCrypto(input: CreateCryptoInput(id: id.isEmpty ? nil : id,
                    date: date, action: action, value: value, valueCurrency: currency, fee: secondary,
                    feeCurrency: secondaryCurrency, provider: provider))
            }
        }
    }
}

@MainActor enum RecordDate {
    static let formatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter
    }()
}

struct RecordEditor: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    @State var draft: RecordDraft
    @State private var saving = false
    @State private var error: String?

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Image(systemName: draft.kind.symbol).foregroundStyle(.secondary)
                Text("\(draft.existingID == nil ? "New" : "Edit") \(draft.kind.singular)").font(.title2.bold())
                Spacer()
            }.padding(24)
            Form {
                Section("Record") {
                    DatePicker("Date", selection: $draft.date, displayedComponents: .date)
                        .environment(\.timeZone, TimeZone(secondsFromGMT: 0)!)
                    TextField("Provider", text: $draft.provider, prompt: Text("Broker or bank"))
                    if draft.kind == .dividend {
                        TextField("Ticker", text: $draft.ticker, prompt: Text("AAPL"))
                        TextField("Country code", text: $draft.country, prompt: Text("US"))
                    }
                    if draft.kind == .crypto {
                        Picker("Action", selection: $draft.action) {
                            Text("Buy").tag(Action.fiatBuy)
                            Text("Sell").tag(Action.fiatSell)
                        }.pickerStyle(.segmented)
                    }
                }
                Section("Amounts") {
                    amountField("Value", value: $draft.value, currency: $draft.currency)
                    if draft.kind != .interest {
                        amountField(draft.kind == .dividend ? "Tax paid" : "Fee",
                                    value: $draft.secondary, currency: $draft.secondaryCurrency)
                    }
                    Text("Enter decimals with a point, for example 123.45.")
                        .font(.caption).foregroundStyle(.secondary)
                }
                Section {
                    TextField("Record ID", text: $draft.customID, prompt: Text("Generated automatically"))
                        .disabled(draft.existingID != nil)
                }
            }.formStyle(.grouped).disabled(saving)
            if let error { Text(error).font(.callout).foregroundStyle(.red).padding(.horizontal, 24).padding(.bottom, 12) }
            HStack {
                if saving { ProgressView().controlSize(.small) }
                Spacer()
                Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction).disabled(saving)
                Button("Save") {
                    Task {
                        saving = true; error = nil
                        defer { saving = false }
                        do {
                            guard try await model.mutate("Record saved.", operation: { core in
                                try await draft.save(using: core)
                            }) else { return }
                            dismiss()
                        } catch { self.error = errorText(error) }
                    }
                }.keyboardShortcut(.defaultAction).disabled(saving || model.isWorking || draft.value.isEmpty)
            }.padding(20)
        }.frame(width: 480, height: draft.kind == .interest ? 450 : 580)
            .interactiveDismissDisabled(saving)
    }

    private func amountField(_ title: String, value: Binding<String>, currency: Binding<Currency>) -> some View {
        LabeledContent(title) {
            HStack {
                TextField(title, text: value, prompt: Text("0.00")).labelsHidden()
                    .multilineTextAlignment(.trailing)
                Picker("Currency for \(title)", selection: currency) {
                    ForEach(Currency.allCases, id: \.self) { Text(currencyCode($0)).tag($0) }
                }.labelsHidden().frame(width: 85)
            }
        }
    }
}
