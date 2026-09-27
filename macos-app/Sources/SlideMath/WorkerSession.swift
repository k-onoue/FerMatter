import Foundation

/// A worker delivers ordered events and its completion on the main queue.
protocol WorkerRunning: AnyObject {
    func run(_ request: [String: Any], receive: @escaping (WorkerEvent) -> Void, finished: @escaping () -> Void)
    func cancel()
}

/// Owns one replaceable request. Events queued by a cancelled or completed
/// worker cannot update the next project, preview, or PDF navigation request.
@MainActor final class WorkerSession {
    private let makeWorker: () -> WorkerRunning
    private var worker: WorkerRunning?
    private var generation: UUID?
    var isRunning: Bool { worker != nil }

    init(makeWorker: @escaping () -> WorkerRunning = { Worker() }) {
        self.makeWorker = makeWorker
    }

    func start(_ request: [String: Any], receive: @escaping (WorkerEvent) -> Void, finished: @escaping () -> Void = {}) {
        cancel()
        let token = UUID()
        generation = token
        let runner = makeWorker()
        worker = runner
        runner.run(request, receive: { [weak self] event in
            guard self?.generation == token else { return }
            receive(event)
        }, finished: { [weak self] in
            guard let self, self.generation == token else { return }
            self.generation = nil
            self.worker = nil
            finished()
        })
    }

    func cancel() {
        generation = nil
        let previous = worker
        worker = nil
        previous?.cancel()
    }

    deinit { worker?.cancel() }
}
