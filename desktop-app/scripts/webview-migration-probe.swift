import AppKit
import WebKit

final class Probe: NSObject, NSApplicationDelegate, WKNavigationDelegate, WKURLSchemeHandler {
    var web: WKWebView!
    let mode = CommandLine.arguments[1]
    func applicationDidFinishLaunching(_ notification: Notification) {
        let config = WKWebViewConfiguration()
        config.setURLSchemeHandler(self, forURLScheme: "tauri")
        web = WKWebView(frame: .zero, configuration: config)
        web.navigationDelegate = self
        web.load(URLRequest(url: URL(string: "tauri://localhost/")!))
        DispatchQueue.main.asyncAfter(deadline: .now() + 25) { exit(3) }
    }
    func webView(_ webView: WKWebView, start urlSchemeTask: WKURLSchemeTask) {
        let data = Data("<!doctype html><title>Migration probe</title>".utf8)
        urlSchemeTask.didReceive(URLResponse(url: urlSchemeTask.request.url!, mimeType: "text/html", expectedContentLength: data.count, textEncodingName: "utf-8"))
        urlSchemeTask.didReceive(data)
        urlSchemeTask.didFinish()
    }
    func webView(_ webView: WKWebView, stop urlSchemeTask: WKURLSchemeTask) {}
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        let script = mode == "write" ? "localStorage.setItem('mathlidelux.migration-probe', 'synthetic-draft'); localStorage.getItem('mathlidelux.migration-probe')" : "localStorage.getItem('mathlidelux.migration-probe')"
        webView.evaluateJavaScript(script) { result, error in
            print(String(describing: result), String(describing: error))
            let code: Int32 = result as? String == "synthetic-draft" && error == nil ? 0 : 2
            DispatchQueue.main.asyncAfter(deadline: .now() + 2) { exit(code) }
        }
    }
}
let app = NSApplication.shared
let delegate = Probe()
app.delegate = delegate
app.setActivationPolicy(.prohibited)
app.run()
