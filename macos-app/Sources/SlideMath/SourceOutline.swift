import SwiftUI

enum SourceOutline {
    static func headings(_ text: String, file: String, format: ProjectFormat) -> [DocumentHeading] {
        var result: [DocumentHeading] = [], parents: [(Int, String)] = []
        let levels = ["part": 0, "chapter": 1, "section": 2, "subsection": 3, "subsubsection": 4, "paragraph": 5, "subparagraph": 6]
        let pattern = format == .latex ? #"\\(part|chapter|section|subsection|subsubsection|paragraph|subparagraph)\*?\s*(?:\[[^\]]*\])?\s*\{"# : #"^(={1,6})\s+(.+?)\s*$"#
        guard let regex = try? NSRegularExpression(pattern: pattern) else { return [] }
        var blockComment = false, verbatim = false
        for (index, original) in text.replacingOccurrences(of: "\r\n", with: "\n").components(separatedBy: .newlines).enumerated() {
            var line = "", previous: Character?
            let chars = Array(original); var offset = 0
            while offset < chars.count {
                let char = chars[offset], next = offset + 1 < chars.count ? chars[offset + 1] : nil
                if format == .typst {
                    if blockComment { if char == "*" && next == "/" { blockComment = false; offset += 2 } else { offset += 1 }; continue }
                    if char == "/" && next == "*" { blockComment = true; offset += 2; continue }
                    if char == "/" && next == "/" { break }
                } else if char == "%" && previous != "\\" { break }
                line.append(char); previous = char; offset += 1
            }
            if format == .latex, line.contains(#"\begin{verbatim}"#) { verbatim = true }
            if format == .typst, line.trimmingCharacters(in: .whitespaces).hasPrefix("```") { verbatim.toggle(); continue }
            if verbatim {
                if format == .latex && line.contains(#"\end{verbatim}"#) { verbatim = false }
                continue
            }
            let source = line as NSString
            for match in regex.matches(in: line, range: NSRange(location: 0, length: source.length)) {
                let level: Int, title: String
                if format == .typst {
                    level = match.range(at: 1).length
                    title = source.substring(with: match.range(at: 2)).replacingOccurrences(of: #"\s*<[^>]+>\s*$"#, with: "", options: .regularExpression)
                } else {
                    level = levels[source.substring(with: match.range(at: 1))] ?? 2
                    let start = NSMaxRange(match.range); var cursor = start, depth = 1
                    while cursor < source.length && depth > 0 {
                        let char = source.character(at: cursor)
                        if char == 123 { depth += 1 }; if char == 125 { depth -= 1 }
                        if depth > 0 { cursor += 1 }
                    }
                    guard depth == 0 else { continue }
                    title = source.substring(with: NSRange(location: start, length: cursor - start))
                        .replacingOccurrences(of: #"\\[A-Za-z]+\*?"#, with: "", options: .regularExpression)
                        .replacingOccurrences(of: "{", with: "").replacingOccurrences(of: "}", with: "")
                }
                let id = "\(file):\(index + 1):\(match.range.location)"
                while let last = parents.last, last.0 >= level { parents.removeLast() }
                parents.append((level, id))
                result.append(DocumentHeading(id: id, title: title, level: level, file: file, line: index + 1, path: parents.map(\.1)))
            }
        }
        return result
    }
}

struct SourceSidebar: View {
    @EnvironmentObject private var store: LibraryStore
    @ObservedObject var files: ProjectFiles
    @State private var documentOutline = false
    @State private var collapsed = Set<String>()
    @State private var expanded = true
    private var headings: [DocumentHeading] {
        if documentOutline { return store.outline }
        guard let current = files.current else { return [] }
        return SourceOutline.headings(current.text, file: current.path, format: current.path.hasSuffix(".typ") ? .typst : files.format)
    }
    var body: some View {
        VSplitView {
            ProjectFileTree(files: files).frame(minHeight: 160)
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Button { expanded.toggle() } label: { Label("Outline", systemImage: expanded ? "chevron.down" : "chevron.right") }
                        .buttonStyle(.plain).font(.system(size: 11, weight: .medium))
                    Spacer()
                    Menu {
                        Picker("Outline", selection: $documentOutline) { Text("Current File").tag(false); Text("Document").tag(true) }
                    } label: { Image(systemName: "ellipsis") }.menuIndicator(.hidden).menuStyle(.borderlessButton).frame(width: 20).help("Outline Scope")
                }.padding(.horizontal, 8).padding(.top, 10)
                if expanded {
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 2) {
                            ForEach(headings.filter { $0.path.dropLast().allSatisfy { !collapsed.contains($0) } }) { heading in
                                HStack(spacing: 4) {
                                    if headings.contains(where: { $0.path.dropLast().last == heading.id }) {
                                        Button { if !collapsed.insert(heading.id).inserted { collapsed.remove(heading.id) } } label: { Image(systemName: collapsed.contains(heading.id) ? "chevron.right" : "chevron.down").font(.system(size: 9)) }.buttonStyle(.plain).frame(width: 12)
                                    } else { Spacer().frame(width: 12) }
                                    Button { files.open(heading.file, line: heading.line) } label: {
                                        Text(heading.title).lineLimit(1).frame(maxWidth: .infinity, alignment: .leading).contentShape(Rectangle())
                                    }.buttonStyle(.plain).help("\(heading.file):\(heading.line)")
                                        .accessibilityLabel("\(heading.title), line \(heading.line)")
                                }.font(.system(size: 12)).padding(.vertical, 6).padding(.leading, CGFloat(max(0, heading.path.count - 1) * 12 + 4))
                            }
                            if headings.isEmpty { Text("No headings").font(.caption).foregroundStyle(.secondary).padding(8) }
                        }
                    }
                }
            }.frame(minHeight: expanded ? 100 : 30, idealHeight: 220)
        }
    }
}
