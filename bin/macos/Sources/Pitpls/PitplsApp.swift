import SwiftUI
import PitCore

@main
struct PitplsApp: App {
    @State private var model = AppModel()

    var body: some Scene {
        Window("Pitpls", id: "main") {
            ContentView()
                .environment(model)
                .standardWindowBackground()
        }
        .defaultSize(width: 1120, height: 740)
        .commands {
            SidebarCommands()
            CommandGroup(after: .newItem) {
                Button("Import Records…") { model.destination = .imports }
                    .keyboardShortcut("i", modifiers: [.command, .shift])
                Button("Refresh") { model.refresh() }
                    .keyboardShortcut("r")
                    .disabled(model.core == nil || model.isWorking)
            }
        }
        SwiftUI.Settings {
            SettingsView()
                .environment(model)
                .standardWindowBackground()
        }
    }
}

extension View {
    @ViewBuilder
    func standardWindowBackground() -> some View {
        if #available(macOS 15.0, *) {
            self.containerBackground(.background, for: .window)
        } else {
            self.background(.background)
        }
    }
}

enum Destination: String, CaseIterable, Identifiable {
    case overview, dividends, interests, crypto, imports, rates
    var id: Self { self }
    var title: String { rawValue.capitalized }
    var symbol: String {
        switch self {
        case .overview: "house"
        case .dividends: "chart.bar.xaxis"
        case .interests: "percent"
        case .crypto: "bitcoinsign.circle"
        case .imports: "square.and.arrow.down"
        case .rates: "chart.xyaxis.line"
        }
    }
    var recordKind: RecordKind? {
        switch self {
        case .dividends: .dividend
        case .interests: .interest
        case .crypto: .crypto
        default: nil
        }
    }
}

@MainActor @Observable
final class AppModel {
    private(set) var core: RustCore?
    private(set) var years: [Int32] = []
    private(set) var revision = 0
    private(set) var isConnecting = false
    private(set) var isWorking = false
    var destination: Destination? = .overview
    var year: Int32? = nil
    var error: String?
    var notice: String?

    // Match Tauri's app_data_dir. A separate bundle ID only isolates UI preferences.
    let databaseURL: URL = {
        if let path = ProcessInfo.processInfo.environment["PITPLS_DATABASE_PATH"] {
            return URL(fileURLWithPath: path)
        }
        return URL.applicationSupportDirectory
            .appending(path: "com.mngapp.pitpls/pitpls.db")
    }()

    var loadID: String { "\(year.map(String.init) ?? "all")-\(revision)" }

    func connect() async {
        guard core == nil, !isConnecting else { return }
        isConnecting = true
        defer { isConnecting = false }
        do {
            let client = try await RustCore.open(databasePath: databaseURL.path)
            years = try await client.listYears()
            core = client
            refresh()
        } catch { self.error = errorText(error) }
    }

    func refresh() { revision += 1 }

    @discardableResult
    func perform(_ success: String? = nil, operation: (RustCore) async throws -> Void) async -> Bool {
        do { return try await mutate(success, operation: operation) }
        catch { self.error = errorText(error); return false }
    }

    // Sheets handle errors locally; all mutations still share the same busy state.
    @discardableResult
    func mutate(_ success: String? = nil, operation: (RustCore) async throws -> Void) async throws -> Bool {
        guard let core, !isWorking else { return false }
        isWorking = true
        defer {
            isWorking = false
            refresh()
        }
        try await operation(core)
        notice = success
        do { years = try await core.listYears() }
        catch { self.error = "The change was saved, but years could not be refreshed. \(errorText(error))" }
        return true
    }
}

func errorText(_ error: Error) -> String {
    if let error = error as? NativeError, case let .Failed(message) = error { return message }
    return error.localizedDescription
}
