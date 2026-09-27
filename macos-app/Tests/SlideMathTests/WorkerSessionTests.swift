import XCTest
@testable import SlideMath

private final class ControlledWorker: WorkerRunning {
    var cancelled = false
    var request: [String: Any] = [:]
    var receive: ((WorkerEvent) -> Void)?
    var finished: (() -> Void)?
    func run(_ request: [String: Any], receive: @escaping (WorkerEvent) -> Void, finished: @escaping () -> Void) {
        self.request = request; self.receive = receive; self.finished = finished
    }
    func cancel() { cancelled = true }
}

final class WorkerSessionTests: XCTestCase {
    @MainActor func testReplacementRejectsAlreadyQueuedEventsAndCompletion() {
        let first = ControlledWorker(), second = ControlledWorker()
        var queue = [first, second]
        let session = WorkerSession { queue.removeFirst() }
        var events: [String] = [], completions: [String] = []
        session.start(["action": "old"], receive: { events.append($0.event) }, finished: { completions.append("old") })
        first.receive?(WorkerEvent(event: "before replacement"))
        session.start(["action": "new"], receive: { events.append($0.event) }, finished: { completions.append("new") })
        XCTAssertTrue(first.cancelled)
        first.receive?(WorkerEvent(event: "stale error")); first.finished?()
        XCTAssertTrue(session.isRunning)
        second.receive?(WorkerEvent(event: "current")); second.finished?()
        second.receive?(WorkerEvent(event: "after completion")); second.finished?()
        XCTAssertEqual(events, ["before replacement", "current"])
        XCTAssertEqual(completions, ["new"])
        XCTAssertEqual(second.request["action"] as? String, "new")
        XCTAssertFalse(session.isRunning)
    }

    @MainActor func testCancellationAndOwnerReleaseStopWorkWithoutDeliveringCallbacks() {
        let worker = ControlledWorker()
        var session: WorkerSession? = WorkerSession { worker }
        var delivered = 0
        session?.start([:], receive: { _ in delivered += 1 }, finished: { delivered += 1 })
        session?.cancel()
        worker.receive?(WorkerEvent(event: "card")); worker.finished?()
        XCTAssertTrue(worker.cancelled); XCTAssertEqual(delivered, 0)
        worker.cancelled = false
        session?.start([:], receive: { _ in delivered += 1 })
        session = nil
        XCTAssertTrue(worker.cancelled)
        worker.receive?(WorkerEvent(event: "card")); worker.finished?()
        XCTAssertEqual(delivered, 0)
    }

    @MainActor func testCompletionCanStartNextRequestWithoutLosingIt() {
        let first = ControlledWorker(), second = ControlledWorker()
        var queue = [first, second]
        let session = WorkerSession { queue.removeFirst() }
        var events: [String] = []
        session.start([:], receive: { _ in }, finished: {
            session.start([:], receive: { events.append($0.event) })
        })
        first.finished?()
        first.finished?() // An old completion must not clear the new request.
        XCTAssertTrue(session.isRunning)
        second.receive?(WorkerEvent(event: "next")); second.finished?()
        XCTAssertEqual(events, ["next"]); XCTAssertFalse(session.isRunning)
    }
}
