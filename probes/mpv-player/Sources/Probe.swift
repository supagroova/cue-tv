import SwiftUI
import Libmpv
import Metal

@main struct Probe: App {
    var body: some Scene { WindowGroup { ProbeView() } }
}
struct ProbeView: View {
    @State private var progress = "Starting"
    @State private var tracks = ""
    var body: some View {
        VStack {
            MetalSurface { progress = $0; tracks = $1 }.frame(width: 960, height: 540)
            Text(progress).accessibilityIdentifier("mpv-progress")
            Text(tracks)
        }
    }
}
final class MetalView: UIView {
    override class var layerClass: AnyClass { CAMetalLayer.self }
    override func layoutSubviews() {
        super.layoutSubviews()
        let metal = layer as! CAMetalLayer
        metal.drawableSize = bounds.size
    }
}
struct MetalSurface: UIViewRepresentable {
    let report: (String, String) -> Void
    func makeCoordinator() -> Coordinator { Coordinator() }
    func makeUIView(context: Context) -> MetalView {
        let view = MetalView()
        let layer = view.layer as! CAMetalLayer
        layer.device = MTLCreateSystemDefaultDevice()
        context.coordinator.start(layer: layer, report: report)
        return view
    }
    func updateUIView(_ view: MetalView, context: Context) {}
    static func dismantleUIView(_ view: MetalView, coordinator: Coordinator) { coordinator.stop() }
    final class Coordinator {
        var handle: OpaquePointer?
        var polling: Task<Void, Never>?
        func start(layer: CAMetalLayer, report: @escaping (String, String) -> Void) {
            guard let handle = mpv_create() else { report("create failed", ""); return }
            self.handle = handle
            mpv_set_option_string(handle, "vo", "gpu-next")
            mpv_set_option_string(handle, "gpu-api", "vulkan")
            mpv_set_option_string(handle, "gpu-context", "moltenvk")
            mpv_set_option_string(handle, "hwdec", "auto-safe")
            var pointer = Int64(Int(bitPattern: Unmanaged.passUnretained(layer).toOpaque()))
            mpv_set_option(handle, "wid", MPV_FORMAT_INT64, &pointer)
            if ProcessInfo.processInfo.environment["CUE_MPV_ALTERNATE"] == "1" {
                mpv_set_option_string(handle, "aid", "2")
                mpv_set_option_string(handle, "sid", "2")
            }
            let result = mpv_initialize(handle)
            guard result >= 0 else { report("initialize failed: \(result)", ""); return }
            let fixture = ProcessInfo.processInfo.environment["CUE_MPV_FIXTURE"] ?? "episode01.mp4"
            guard let url = Bundle.main.url(forResource: fixture, withExtension: nil) else { report("missing fixture", ""); return }
            let command: [String] = ["loadfile", url.path]
            let strings = command.map { value in value.withCString { strdup($0) } }
            defer { strings.forEach { free($0) } }
            var args = strings.map { UnsafePointer<CChar>($0) } + [nil]
            args.withUnsafeMutableBufferPointer { _ = mpv_command(handle, $0.baseAddress) }
            polling = Task { @MainActor in
                while !Task.isCancelled {
                    while mpv_wait_event(handle, 0)?.pointee.event_id != MPV_EVENT_NONE {}
                    var time = 0.0
                    let result = mpv_get_property(handle, "time-pos", MPV_FORMAT_DOUBLE, &time)
                    var audio: Int64 = 0
                    var subtitle: Int64 = 0
                    mpv_get_property(handle, "aid", MPV_FORMAT_INT64, &audio)
                    mpv_get_property(handle, "sid", MPV_FORMAT_INT64, &subtitle)
                    report(result >= 0 ? String(format: "%.2f", time) : "Waiting: \(result)",
                           "Selected audio: \(audio); subtitle: \(subtitle)")
                    try? await Task.sleep(for: .milliseconds(100))
                }
            }
        }
        func stop() {
            polling?.cancel(); polling = nil
            if let handle { mpv_terminate_destroy(handle) }
            handle = nil
        }
    }
}
