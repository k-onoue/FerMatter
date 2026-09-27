import XCTest
import AppKit
@testable import SlideMath

final class WorkspaceInteractionTests: XCTestCase {
    @MainActor func testPDFAndInspectorWidthsRestoreIndependentlyAfterWindowShrinks() {
        let suite = "PaneTests-\(UUID())"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let widths = PaneWidths(defaults: defaults)
        widths.set(720, for: .pdf); widths.set(340, for: .inspector)
        XCTAssertEqual(widths.fitted(.pdf, available: 1800), 720)
        XCTAssertEqual(widths.fitted(.inspector, available: 1800), 340)
        XCTAssertEqual(widths.fitted(.pdf, available: 1040), 350)
        XCTAssertEqual(widths.fitted(.pdf, available: 1800), 720)
        XCTAssertEqual(PaneWidths(defaults: defaults).preferred(.pdf), 720)
        XCTAssertEqual(PaneWidths(defaults: defaults).preferred(.inspector), 340)
    }

    @MainActor func testFinderSelectionToggleRangesSelectAllAndFiltering() {
        let store = LibraryStore(support: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
        store.cards = (0..<8).map { Equation(id: "\($0)", name: "\($0)", tex: "x", kind: $0 < 5 ? "inline" : "block", origin: "source", sourceOrder: Double($0), status: "pending") }
        store.selectCard("1"); store.selectCard("3", modifiers: .command)
        XCTAssertEqual(store.selectedIDs, ["1", "3"])
        store.selectCard("3", modifiers: .command)
        XCTAssertEqual(store.selectedIDs, ["1"])
        store.selectCard("2"); store.selectCard("5", modifiers: .shift)
        XCTAssertEqual(store.selectedIDs, ["2", "3", "4", "5"])
        store.selectCard("4", modifiers: .shift)
        XCTAssertEqual(store.selectedIDs, ["2", "3", "4"])
        store.selectAllCards(); XCTAssertEqual(store.selectedIDs.count, 8)
        store.kind = "inline"; store.retainVisibleSelection()
        XCTAssertEqual(store.selectedIDs, ["0", "1", "2", "3", "4"])
        store.setCardSelection(["2", "7", "missing"])
        XCTAssertEqual(store.selectedIDs, ["2"])
        store.clearCardSelection(); XCTAssertTrue(store.selectedIDs.isEmpty); XCTAssertNil(store.selection)
    }
}
