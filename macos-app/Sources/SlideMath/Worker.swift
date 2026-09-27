import Foundation

final class Worker: WorkerRunning, @unchecked Sendable {
    private let lock = NSLock()
    private var process: Process?
    private var cancelled = false

    func cancel() {
        lock.lock(); cancelled = true
        if let process, process.isRunning { process.terminate() }
        lock.unlock()
    }

    func run(_ request: [String: Any], receive: @escaping (WorkerEvent) -> Void, finished: @escaping () -> Void) {
        DispatchQueue.global(qos: .userInitiated).async {
            defer { DispatchQueue.main.async(execute: finished) }
            let input = FileManager.default.temporaryDirectory.appendingPathComponent("slide-math-\(UUID()).json")
            defer { try? FileManager.default.removeItem(at: input) }
            do {
                try JSONSerialization.data(withJSONObject: request).write(to: input, options: .atomic)
                guard let executable = ["/opt/homebrew/bin/python3", "/usr/local/bin/python3", "/usr/bin/python3"].first(where: { FileManager.default.isExecutableFile(atPath: $0) }) else {
                    throw NSError(domain: "SlideMath", code: 1, userInfo: [NSLocalizedDescriptionKey: "Python 3 is required. Install Python 3, then try again."])
                }
                let worker = Bundle.main.resourceURL!.appendingPathComponent("tex_worker.py")
                guard FileManager.default.fileExists(atPath: worker.path) else { throw CocoaError(.fileNoSuchFile) }
                let task = Process(), pipe = Pipe(), errors = Pipe()
                task.executableURL = URL(fileURLWithPath: executable)
                // Imported workers must never write __pycache__ into the signed bundle.
                task.arguments = ["-B", worker.path, input.path]
                task.standardOutput = pipe
                task.standardError = errors
                // The worker reports failures as JSON; drain incidental Python diagnostics.
                errors.fileHandleForReading.readabilityHandler = { handle in _ = handle.availableData }
                self.lock.lock()
                if self.cancelled { self.lock.unlock(); return }
                self.process = task
                do { try task.run() } catch { self.process = nil; self.lock.unlock(); throw error }
                self.lock.unlock()
                var buffer = Data()
                var reportedError = false
                while true {
                    let data = pipe.fileHandleForReading.availableData
                    if data.isEmpty { break }
                    buffer.append(data)
                    while let newline = buffer.firstIndex(of: 10) {
                        let line = buffer.prefix(upTo: newline)
                        buffer.removeSubrange(...newline)
                        if let event = try? JSONDecoder().decode(WorkerEvent.self, from: line) {
                            if event.event == "error" { reportedError = true }
                            DispatchQueue.main.async { receive(event) }
                        }
                    }
                }
                task.waitUntilExit()
                errors.fileHandleForReading.readabilityHandler = nil
                self.lock.lock(); self.process = nil; let cancelled = self.cancelled; self.lock.unlock()
                if task.terminationStatus != 0 && !cancelled && !reportedError {
                    let event = WorkerEvent(event: "error", message: "Rendering failed. Check the log and compiler settings.")
                    DispatchQueue.main.async { receive(event) }
                }
            } catch {
                let event = WorkerEvent(event: "error", message: error.localizedDescription)
                DispatchQueue.main.async { receive(event) }
            }
        }
    }
}
