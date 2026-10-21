import AppKit
import Darwin
import Foundation
import WebKit

let profileStart = ProcessInfo.processInfo.systemUptime
func profile(_ stage: String) {
    if ProcessInfo.processInfo.environment["MDV_PROFILE_STARTUP"] != nil {
        fputs("snapshot.\(stage) \((ProcessInfo.processInfo.systemUptime - profileStart) * 1000)ms\n", stderr)
    }
}

final class SnapshotRunner: NSObject, WKNavigationDelegate {
    private let htmlURL: URL
    private let readAccessURL: URL
    private let outputURL: URL
    private let reportURL: URL
    private let viewportWidth: CGFloat
    private let outputFormat: String
    private(set) var done = false
    private(set) var failure: String?
    private var latestProbe: [String: Any] = [:]
    private var lastMeasuredHeight: Double = -1
    private var stableProbeCount = 0
    private let completion: ((String?) -> Void)?
    private var hasCompleted = false
    private var navigation: WKNavigation?
    private let webView: WKWebView

    init(
        htmlURL: URL,
        readAccessURL: URL,
        outputURL: URL,
        reportURL: URL,
        viewportWidth: CGFloat,
        outputFormat: String = "png",
        webView: WKWebView? = nil,
        completion: ((String?) -> Void)? = nil
    ) {
        self.htmlURL = htmlURL
        self.readAccessURL = readAccessURL
        self.outputURL = outputURL
        self.reportURL = reportURL
        self.viewportWidth = viewportWidth
        self.outputFormat = outputFormat
        self.completion = completion
        if let webView {
            self.webView = webView
        } else {
            self.webView = WKWebView(
                frame: CGRect(x: 0, y: 0, width: viewportWidth, height: 900),
                configuration: WKWebViewConfiguration()
            )
        }
        super.init()
        self.webView.navigationDelegate = self
    }

    func start() {
        profile("start")
        _ = NSApplication.shared
        profile("application_ready")
        _ = webView
        profile("webview_ready")
        webView.setFrameSize(NSSize(width: viewportWidth, height: 900))
        navigation = webView.loadFileURL(htmlURL, allowingReadAccessTo: readAccessURL)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        guard !done, navigation === self.navigation else { return }
        profile("navigation_finished")
        self.awaitVisualStability(attempt: 0)
    }

    func webView(
        _ webView: WKWebView,
        didFail navigation: WKNavigation!,
        withError error: Error
    ) {
        guard !done, navigation === self.navigation else { return }
        fail(error.localizedDescription)
    }

    func webView(
        _ webView: WKWebView,
        didFailProvisionalNavigation navigation: WKNavigation!,
        withError error: Error
    ) {
        guard !done, navigation === self.navigation else { return }
        fail(error.localizedDescription)
    }

    func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
        fail("web content process terminated")
    }

    func cancel(_ reason: String) {
        guard !done else { return }
        webView.stopLoading()
        finish(reason)
    }

    private func awaitVisualStability(attempt: Int) {
        guard !done else { return }
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
            guard !self.done else { return }
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
        guard !done else { return }
        profile("visuals_ready")
        let configuration = WKSnapshotConfiguration()
        configuration.rect = CGRect(x: 0, y: 0, width: viewportWidth, height: height)
        configuration.afterScreenUpdates = true

        webView.takeSnapshot(with: configuration) { image, error in
            guard !self.done else { return }
            profile("snapshot_returned")
            if let error {
                self.fail(error.localizedDescription)
                return
            }
            guard
                let image,
                let cgImage = image.cgImage(forProposedRect: nil, context: nil, hints: nil)
            else {
                self.fail("failed to obtain snapshot pixels")
                return
            }
            let bitmap = NSBitmapImageRep(cgImage: cgImage)
            let data = self.outputFormat == "tiff"
                ? bitmap.representation(using: .tiff, properties: [.compressionMethod: 1])
                : bitmap.representation(using: .png, properties: [:])
            guard let data else {
                self.fail("failed to encode snapshot raster")
                return
            }

            do {
                profile("raster_encoded")
                try data.write(to: self.outputURL)
                try self.writeDiagnosticsReport()
                profile("complete")
                self.finish()
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
        finish(message)
    }

    private func finish(_ failure: String? = nil) {
        guard !hasCompleted else {
            return
        }
        hasCompleted = true
        self.failure = failure
        self.done = true
        completion?(failure)
    }
}

private struct SnapshotRequest: Decodable {
    let htmlPath: String
    let readAccessRoot: String
    let outputPath: String
    let reportPath: String
    let width: UInt32
    let outputFormat: String?
    let timeoutMs: UInt32?

    enum CodingKeys: String, CodingKey {
        case htmlPath = "html_path"
        case readAccessRoot = "read_access_root"
        case outputPath = "output_path"
        case reportPath = "report_path"
        case width
        case outputFormat = "output_format"
        case timeoutMs = "timeout_ms"
    }
}

private struct SnapshotResponse: Encodable {
    let ok: Bool
    let error: String?

    enum CodingKeys: String, CodingKey {
        case ok
        case error
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(ok, forKey: .ok)
        if let error {
            try container.encode(error, forKey: .error)
        }
    }
}

private final class SnapshotServer {
    private static let maximumRequestBytes = 64 * 1024
    private static let idleTimeout: TimeInterval = 120

    private let socketPath: String
    private var listener: Int32
    private let ioQueue = DispatchQueue(label: "mdv.snapshot.server")
    private let processPool = WKProcessPool()
    private let dataStore = WKWebsiteDataStore.nonPersistent()
    private var reusableWebView: WKWebView?
    private var currentRunner: SnapshotRunner?
    private var idleTimer: Timer?
    private var renderTimer: Timer?
    private var rendering = false
    private(set) var isStopped = false

    init(socketPath: String) throws {
        self.socketPath = socketPath
        self.listener = try Self.makeListener(socketPath: socketPath)
    }

    deinit {
        if listener >= 0 {
            Darwin.close(listener)
        }
    }

    func start() {
        let application = NSApplication.shared
        application.setActivationPolicy(.prohibited)
        profile("server_ready")
        armIdleTimer()
        beginNextAccept()
    }

    private static func makeListener(socketPath: String) throws -> Int32 {
        let descriptor = Darwin.socket(AF_UNIX, SOCK_STREAM, 0)
        guard descriptor >= 0 else {
            throw POSIXError(POSIXErrorCode(rawValue: errno) ?? .EIO)
        }

        do {
            let pathBytes = Array(socketPath.utf8)
            var address = sockaddr_un()
            let maximumPathBytes = MemoryLayout.size(ofValue: address.sun_path) - 1
            guard !pathBytes.isEmpty && pathBytes.count <= maximumPathBytes else {
                throw NSError(
                    domain: "mdv.snapshot",
                    code: Int(ENAMETOOLONG),
                    userInfo: [NSLocalizedDescriptionKey: "unix socket path is too long"]
                )
            }

            _ = socketPath.withCString { Darwin.unlink($0) }
            address.sun_len = UInt8(MemoryLayout<sockaddr_un>.size)
            address.sun_family = sa_family_t(AF_UNIX)
            withUnsafeMutableBytes(of: &address.sun_path) { destination in
                destination.initializeMemory(as: UInt8.self, repeating: 0)
                pathBytes.withUnsafeBytes { source in
                    destination.copyBytes(from: source)
                }
            }
            let bindResult = withUnsafePointer(to: &address) { pointer in
                pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                    Darwin.bind(descriptor, $0, socklen_t(MemoryLayout<sockaddr_un>.size))
                }
            }
            guard bindResult == 0 else {
                throw POSIXError(POSIXErrorCode(rawValue: errno) ?? .EIO)
            }
            guard Darwin.listen(descriptor, SOMAXCONN) == 0 else {
                throw POSIXError(POSIXErrorCode(rawValue: errno) ?? .EIO)
            }
            return descriptor
        } catch {
            Darwin.close(descriptor)
            throw error
        }
    }

    private func beginNextAccept() {
        guard !isStopped, listener >= 0 else {
            return
        }
        let listener = listener
        ioQueue.async { [weak self, listener] in
            guard let self else {
                return
            }
            let connection = Darwin.accept(listener, nil, nil)
            guard connection >= 0 else {
                let acceptError = errno
                DispatchQueue.main.async {
                    self.handleAcceptError(acceptError)
                }
                return
            }
            var noSigPipe: Int32 = 1
            _ = withUnsafePointer(to: &noSigPipe) {
                Darwin.setsockopt(
                    connection,
                    SOL_SOCKET,
                    SO_NOSIGPIPE,
                    $0,
                    socklen_t(MemoryLayout<Int32>.size)
                )
            }
            self.readRequest(from: connection)
        }
    }

    private func handleAcceptError(_ error: Int32) {
        guard !isStopped else {
            return
        }
        if error == EINTR {
            beginNextAccept()
            return
        }
        fputs("snapshot server accept failed: \(String(cString: strerror(error)))\n", stderr)
        stop()
    }

    private func readRequest(from connection: Int32) {
        var requestData = Data()
        var chunk = [UInt8](repeating: 0, count: 4096)

        while true {
            let count = chunk.withUnsafeMutableBytes { bytes in
                Darwin.read(connection, bytes.baseAddress, bytes.count)
            }
            if count == 0 {
                if requestData.isEmpty {
                    Darwin.close(connection)
                    DispatchQueue.main.async {
                        self.connectionFinished()
                    }
                } else {
                    replyThenContinue(
                        on: connection,
                        response: SnapshotResponse(
                            ok: false,
                            error: "request must be a newline-delimited JSON object"
                        )
                    )
                }
                return
            }
            if count < 0 {
                if errno == EINTR {
                    continue
                }
                Darwin.close(connection)
                DispatchQueue.main.async {
                    self.connectionFinished()
                }
                return
            }

            requestData.append(chunk, count: count)
            if requestData.count > Self.maximumRequestBytes {
                replyThenContinue(
                    on: connection,
                    response: SnapshotResponse(ok: false, error: "request exceeds 65536 bytes")
                )
                return
            }
            if let newline = requestData.firstIndex(of: 0x0A) {
                var line = requestData[..<newline]
                if line.last == 0x0D {
                    line = line.dropLast()
                }
                do {
                    let request = try JSONDecoder().decode(SnapshotRequest.self, from: line)
                    DispatchQueue.main.async {
                        self.beginRender(request, connection: connection)
                    }
                } catch {
                    replyThenContinue(
                        on: connection,
                        response: SnapshotResponse(ok: false, error: "invalid request: \(error.localizedDescription)")
                    )
                }
                return
            }
        }
    }

    private func beginRender(_ request: SnapshotRequest, connection: Int32) {
        guard
            !request.htmlPath.isEmpty,
            !request.readAccessRoot.isEmpty,
            !request.outputPath.isEmpty,
            !request.reportPath.isEmpty,
            request.width > 0,
            request.outputFormat == nil || ["png", "tiff"].contains(request.outputFormat ?? ""),
            (1...25_000).contains(request.timeoutMs ?? 25_000)
        else {
            replyThenContinue(
                on: connection,
                response: SnapshotResponse(ok: false, error: "request contains an empty path or zero width")
            )
            return
        }

        rendering = true
        idleTimer?.invalidate()
        idleTimer = nil
        let webView = reusableWebView ?? makeReusableWebView(width: request.width)
        reusableWebView = webView
        let runner = SnapshotRunner(
            htmlURL: URL(fileURLWithPath: request.htmlPath),
            readAccessURL: URL(fileURLWithPath: request.readAccessRoot, isDirectory: true),
            outputURL: URL(fileURLWithPath: request.outputPath),
            reportURL: URL(fileURLWithPath: request.reportPath),
            viewportWidth: CGFloat(request.width),
            outputFormat: request.outputFormat ?? "png",
            webView: webView
        ) { [weak self] failure in
            DispatchQueue.main.async {
                self?.completeRender(on: connection, failure: failure)
            }
        }
        currentRunner = runner
        renderTimer = Timer.scheduledTimer(withTimeInterval: Double(request.timeoutMs ?? 25_000) / 1000, repeats: false) { [weak self, weak runner] _ in
            guard let self, let runner, self.currentRunner === runner else { return }
            runner.cancel("snapshot rendering timed out")
        }

        dataStore.removeData(ofTypes: WKWebsiteDataStore.allWebsiteDataTypes(), modifiedSince: .distantPast) { [weak self, weak runner] in
            DispatchQueue.main.async {
                guard let self, let runner, !self.isStopped, !runner.done else {
                    return
                }
                runner.start()
            }
        }
    }

    private func makeReusableWebView(width: UInt32) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.processPool = processPool
        configuration.websiteDataStore = dataStore
        return WKWebView(
            frame: CGRect(x: 0, y: 0, width: CGFloat(width), height: 900),
            configuration: configuration
        )
    }

    private func completeRender(on connection: Int32, failure: String?) {
        renderTimer?.invalidate()
        renderTimer = nil
        if failure != nil {
            reusableWebView?.navigationDelegate = nil
            reusableWebView = nil
        }
        rendering = false
        let response = SnapshotResponse(ok: failure == nil, error: failure)
        ioQueue.async { [weak self] in
            self?.write(response, to: connection)
            Darwin.close(connection)
            DispatchQueue.main.async {
                self?.connectionFinished()
            }
        }
    }

    private func replyThenContinue(on connection: Int32, response: SnapshotResponse) {
        ioQueue.async { [weak self] in
            self?.write(response, to: connection)
            Darwin.close(connection)
            DispatchQueue.main.async {
                self?.connectionFinished()
            }
        }
    }

    private func write(_ response: SnapshotResponse, to connection: Int32) {
        guard var responseData = try? JSONEncoder().encode(response) else {
            return
        }
        responseData.append(0x0A)
        responseData.withUnsafeBytes { bytes in
            guard let baseAddress = bytes.baseAddress else {
                return
            }
            var offset = 0
            while offset < bytes.count {
                let count = Darwin.write(connection, baseAddress.advanced(by: offset), bytes.count - offset)
                if count > 0 {
                    offset += count
                    continue
                }
                if count < 0 && errno == EINTR {
                    continue
                }
                return
            }
        }
    }

    private func connectionFinished() {
        currentRunner = nil
        guard !isStopped else {
            return
        }
        armIdleTimer()
        beginNextAccept()
    }

    private func armIdleTimer() {
        guard !isStopped && !rendering else {
            return
        }
        idleTimer?.invalidate()
        idleTimer = Timer.scheduledTimer(withTimeInterval: Self.idleTimeout, repeats: false) { [weak self] _ in
            self?.stop()
        }
    }

    private func stop() {
        guard !isStopped, !rendering else {
            return
        }
        isStopped = true
        idleTimer?.invalidate()
        idleTimer = nil
        let listener = listener
        self.listener = -1
        Darwin.close(listener)
        _ = socketPath.withCString { Darwin.unlink($0) }
    }
}

let arguments = CommandLine.arguments
if arguments.count == 3 && arguments[1] == "--serve" {
    do {
        let server = try SnapshotServer(socketPath: arguments[2])
        server.start()
        while !server.isStopped && RunLoop.current.run(mode: .default, before: Date(timeIntervalSinceNow: 0.05)) {
        }
        exit(0)
    } catch {
        fputs("snapshot server failed: \(error.localizedDescription)\n", stderr)
        exit(2)
    }
}

guard arguments.count == 6 else {
    fputs("usage: snapshot.swift --serve <unix_socket_path>\n       snapshot.swift <html> <read_access_root> <output_png> <report_json> <width>\n", stderr)
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
