#[cfg(target_os = "macos")]
pub(super) const SWIFT_SNAPSHOT_SCRIPT: &str = r#"
import AppKit
import Foundation
import WebKit

final class SnapshotRunner: NSObject, WKNavigationDelegate {
    private let htmlURL: URL
    private let readAccessURL: URL
    private let outputURL: URL
    private let reportURL: URL
    private let viewportWidth: CGFloat
    private(set) var done = false
    private(set) var failure: String?
    private var latestProbe: [String: Any] = [:]
    private var lastMeasuredHeight: Double = -1
    private var stableProbeCount = 0

    private lazy var webView: WKWebView = {
        let configuration = WKWebViewConfiguration()
        let view = WKWebView(
            frame: CGRect(x: 0, y: 0, width: viewportWidth, height: 900),
            configuration: configuration
        )
        view.navigationDelegate = self
        return view
    }()

    init(
        htmlURL: URL,
        readAccessURL: URL,
        outputURL: URL,
        reportURL: URL,
        viewportWidth: CGFloat
    ) {
        self.htmlURL = htmlURL
        self.readAccessURL = readAccessURL
        self.outputURL = outputURL
        self.reportURL = reportURL
        self.viewportWidth = viewportWidth
        super.init()
    }

    func start() {
        _ = NSApplication.shared
        webView.loadFileURL(htmlURL, allowingReadAccessTo: readAccessURL)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        self.awaitVisualStability(attempt: 0)
    }

    func webView(
        _ webView: WKWebView,
        didFail navigation: WKNavigation!,
        withError error: Error
    ) {
        fail(error.localizedDescription)
    }

    func webView(
        _ webView: WKWebView,
        didFailProvisionalNavigation navigation: WKNavigation!,
        withError error: Error
    ) {
        fail(error.localizedDescription)
    }

    private func awaitVisualStability(attempt: Int) {
        let script = """
        (() => {
          const height = Math.max(
            document.documentElement.scrollHeight || 0,
            document.body.scrollHeight || 0,
            document.documentElement.offsetHeight || 0,
            document.body.offsetHeight || 0
          );
          const fontsReady = !document.fonts || document.fonts.status === "loaded";
          const proseFontReady = !document.fonts || document.fonts.check('16px "Mona Sans VF"');
          const firstHeading = document.querySelector("h1, h2, h3, h4, h5, h6");
          const firstStrong = document.querySelector("strong, b");
          const isRemoteAsset = (value) => /^https?:\\/\\//i.test(value || "");
          const typographySelectors = [
            ["h1", "h1"],
            ["h2", "h2"],
            ["h3", "h3"],
            ["h4", "h4"],
            ["h5", "h5"],
            ["h6", "h6"],
            ["strong", "strong, b"],
            ["em", "em, i"],
            ["code", "p code, li code, blockquote code, td code, th code"]
          ];
          const typography = typographySelectors.map(([role, selector]) => {
            const element = document.querySelector(selector);
            const style = element ? getComputedStyle(element) : null;
            return {
              role,
              present: !!element,
              fontFamily: style ? style.fontFamily || "" : "",
              fontWeight: style ? style.fontWeight || "" : "",
              fontStyle: style ? style.fontStyle || "" : "",
              fontSize: style ? Number.parseFloat(style.fontSize || "0") || 0 : 0,
              lineHeight: style ? Number.parseFloat(style.lineHeight || "0") || 0 : 0
            };
          });
          const images = Array.from(document.images || []).map((image) => {
            const rect = image.getBoundingClientRect();
            const source = image.getAttribute("src") || "";
            const currentSrc = image.currentSrc || "";
            const resolvedSource = currentSrc || source;
            const remote = isRemoteAsset(resolvedSource);
            return {
              source,
              currentSrc,
              complete: !!image.complete,
              naturalWidth: image.naturalWidth || 0,
              naturalHeight: image.naturalHeight || 0,
              renderedWidth: rect.width || 0,
              renderedHeight: rect.height || 0,
              blocking: !remote,
              viewBox: "",
              contentLength: 0
            };
          });
          const mermaids = Array.from(document.querySelectorAll(".mdv-mermaid-diagram")).map((diagram) => {
            const rect = diagram.getBoundingClientRect();
            const viewBox = diagram.getAttribute("viewBox") || "";
            const viewBoxParts = viewBox.split(/[\\s,]+/).map((part) => Number(part));
            const naturalWidth = Number.isFinite(viewBoxParts[2]) ? viewBoxParts[2] : (rect.width || 0);
            const naturalHeight = Number.isFinite(viewBoxParts[3]) ? viewBoxParts[3] : (rect.height || 0);
            return {
              source: viewBox ? `viewBox=${viewBox}` : "mermaid",
              currentSrc: "",
              complete: true,
              naturalWidth,
              naturalHeight,
              renderedWidth: rect.width || 0,
              renderedHeight: rect.height || 0,
              viewBox,
              contentLength: (diagram.innerHTML || "").length
            };
          });
          const imagesReady = images.every((image) => {
            if (!image.blocking) return true;
            if (!image.complete) return false;
            if (!image.currentSrc) return true;
            return image.naturalWidth > 0;
          });
          const remoteImagesReady = images.every((image) => (
            !image.blocking || image.complete
          ));
          const mermaidsReady = mermaids.every((diagram) => (
            diagram.renderedWidth > 0 && diagram.renderedHeight > 0
          ));
          return {
            height,
            fontsReady,
            proseFontReady,
            imagesReady,
            remoteImagesReady,
            mermaidsReady,
            headingFontWeight: firstHeading ? getComputedStyle(firstHeading).fontWeight || "" : "",
            strongFontWeight: firstStrong ? getComputedStyle(firstStrong).fontWeight || "" : "",
            typography,
            images,
            mermaids
          };
        })()
        """
        webView.evaluateJavaScript(script) { result, error in
            if let error {
                self.fail(error.localizedDescription)
                return
            }
            guard let payload = result as? [String: Any] else {
                self.fail("unexpected readiness payload")
                return
            }
            self.latestProbe = payload

            let measuredHeight = (payload["height"] as? NSNumber)?.doubleValue ?? 900.0
            let snapshotHeight = max(CGFloat(ceil(measuredHeight)), 1.0)
            self.webView.setFrameSize(NSSize(width: self.viewportWidth, height: snapshotHeight))

            let fontsReady = (payload["fontsReady"] as? Bool) ?? true
            let imagesReady = (payload["imagesReady"] as? Bool) ?? true
            let remoteImagesReady = (payload["remoteImagesReady"] as? Bool) ?? true
            let mermaidsReady = (payload["mermaidsReady"] as? Bool) ?? true
            let visualsReady = fontsReady && imagesReady && mermaidsReady && (remoteImagesReady || attempt >= 8)
            let currentHeight = Double(snapshotHeight)
            if visualsReady {
                if abs(self.lastMeasuredHeight - currentHeight) < 0.5 {
                    self.stableProbeCount += 1
                } else {
                    self.stableProbeCount = 0
                }
            } else {
                self.stableProbeCount = 0
            }
            self.lastMeasuredHeight = currentHeight

            if (visualsReady && self.stableProbeCount >= 1) || attempt >= 40 {
                if let failure = self.assetFailureMessage(from: payload) {
                    self.fail(failure)
                    return
                }
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.02) {
                    self.takeSnapshot(height: snapshotHeight)
                }
            } else {
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.025) {
                    self.awaitVisualStability(attempt: attempt + 1)
                }
            }
        }
    }

    private func takeSnapshot(height: CGFloat) {
        let configuration = WKSnapshotConfiguration()
        configuration.rect = CGRect(x: 0, y: 0, width: viewportWidth, height: height)
        configuration.afterScreenUpdates = true

        webView.takeSnapshot(with: configuration) { image, error in
            if let error {
                self.fail(error.localizedDescription)
                return
            }
            guard
                let image,
                let tiff = image.tiffRepresentation,
                let bitmap = NSBitmapImageRep(data: tiff),
                let pngData = bitmap.representation(using: .png, properties: [:])
            else {
                self.fail("failed to encode snapshot png")
                return
            }

            do {
                try pngData.write(to: self.outputURL)
                try self.writeDiagnosticsReport()
                self.done = true
            } catch {
                self.fail(error.localizedDescription)
            }
        }
    }

    private func writeDiagnosticsReport() throws {
        let data = try JSONSerialization.data(withJSONObject: latestProbe, options: [.prettyPrinted])
        try data.write(to: reportURL)
    }

    private func assetFailureMessage(from payload: [String: Any]) -> String? {
        var issues: [String] = []

        if let images = payload["images"] as? [[String: Any]] {
            for image in images {
                let blocking = (image["blocking"] as? Bool) ?? true
                if !blocking {
                    continue
                }
                let sourceAttr = (image["source"] as? String) ?? ""
                let currentSrc = (image["currentSrc"] as? String) ?? ""
                let source = !sourceAttr.isEmpty ? sourceAttr : (!currentSrc.isEmpty ? currentSrc : "image")
                let naturalWidth = (image["naturalWidth"] as? NSNumber)?.doubleValue ?? 0
                let complete = (image["complete"] as? Bool) ?? false
                if !complete || (!currentSrc.isEmpty && naturalWidth <= 0) {
                    let renderedWidth = (image["renderedWidth"] as? NSNumber)?.doubleValue ?? 0
                    let renderedHeight = (image["renderedHeight"] as? NSNumber)?.doubleValue ?? 0
                    issues.append("image \(source) failed to render (complete=\(complete), naturalWidth=\(naturalWidth), rendered=\(renderedWidth)x\(renderedHeight))")
                }
            }
        }

        if issues.isEmpty {
            return nil
        }
        return issues.joined(separator: "; ")
    }

    private func fail(_ message: String) {
        self.failure = message
        self.done = true
    }
}

let arguments = CommandLine.arguments
guard arguments.count == 6 else {
    fputs("usage: snapshot.swift <html> <read_access_root> <output_png> <report_json> <width>\n", stderr)
    exit(2)
}

let htmlURL = URL(fileURLWithPath: arguments[1])
let readAccessURL = URL(fileURLWithPath: arguments[2], isDirectory: true)
let outputURL = URL(fileURLWithPath: arguments[3])
let reportURL = URL(fileURLWithPath: arguments[4])
guard let viewportWidth = Double(arguments[5]) else {
    fputs("invalid width\n", stderr)
    exit(2)
}

let runner = SnapshotRunner(
    htmlURL: htmlURL,
    readAccessURL: readAccessURL,
    outputURL: outputURL,
    reportURL: reportURL,
    viewportWidth: CGFloat(viewportWidth)
)
runner.start()

while !runner.done && RunLoop.current.run(mode: .default, before: Date(timeIntervalSinceNow: 0.05)) {
}

if let failure = runner.failure {
    fputs("snapshot failed: \(failure)\n", stderr)
    exit(1)
}
"#;
