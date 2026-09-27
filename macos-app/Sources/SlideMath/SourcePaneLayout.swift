import Foundation

enum SourceSplitAxis: String, Codable { case right, down }

/// A split replaces one leaf, so every pane can be split independently in either direction.
indirect enum SourcePaneLayout: Equatable {
    case pane(Int)
    case split(SourceSplitAxis, SourcePaneLayout, SourcePaneLayout)

    var panes: [Int] {
        switch self {
        case .pane(let id): [id]
        case .split(_, let first, let second): first.panes + second.panes
        }
    }
    func splitting(_ pane: Int, axis: SourceSplitAxis, adding newPane: Int, before: Bool = false) -> Self {
        switch self {
        case .pane(let id): id == pane ? (before ? .split(axis, .pane(newPane), self) : .split(axis, self, .pane(newPane))) : self
        case .split(let direction, let first, let second):
            .split(direction, first.splitting(pane, axis: axis, adding: newPane, before: before), second.splitting(pane, axis: axis, adding: newPane, before: before))
        }
    }
    func removing(_ pane: Int) -> Self? {
        switch self {
        case .pane(let id): return id == pane ? nil : self
        case .split(let direction, let first, let second):
            let a = first.removing(pane), b = second.removing(pane)
            if let a, let b { return .split(direction, a, b) }
            return a ?? b
        }
    }
}
