import AppKit
import SwiftUI

final class CompletionPanel: NSPanel {
    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }
}

struct CompletionList: View {
    let items: [SourceCompletion]
    let selected: Int
    let choose: (Int) -> Void
    var body: some View {
        VStack(spacing: 0) {
            ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
                Button { choose(index) } label: {
                    HStack(spacing: 12) {
                        Text(item.trigger).font(.system(size: 12, design: .monospaced))
                        Spacer(minLength: 4)
                        Text(item.title).font(.system(size: 11)).lineLimit(1).truncationMode(.middle)
                    }.padding(.horizontal, 10).frame(height: 30)
                        .foregroundStyle(index == selected ? Color.white : Color.primary)
                        .background(index == selected ? Color.accentColor : Color.clear, in: RoundedRectangle(cornerRadius: 5))
                }.buttonStyle(.plain).accessibilityLabel(item.trigger + ", " + item.title)
            }
        }.padding(6).background(Color(nsColor: .controlBackgroundColor), in: RoundedRectangle(cornerRadius: 9))
            .overlay(RoundedRectangle(cornerRadius: 9).strokeBorder(.primary.opacity(0.12)))
    }
}
