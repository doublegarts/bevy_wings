// SwiftUI reference app for the Wings Apple designs.
//
// It renders the SwiftUI counterpart of every Wings widget and prints measurements, so the
// Wings iOS and macOS skins can be checked against what SwiftUI really draws:
//
//   METRIC <name> <width> <height>      frame of a control
//   COLOR  <name> <light|dark> r g b a  resolved system color (sRGB, 0..1)
//   FONT   <style> <size> <lineHeight> <weight> <fontName>
//
// The first launch argument selects a page: controls (default), list, tabs, alert.
// Build and run with build-macos.sh or build-ios.sh. Not part of the published crates.

import SwiftUI

#if os(iOS)
import UIKit
typealias PlatformColor = UIColor
#else
import AppKit
typealias PlatformColor = NSColor
#endif

// MARK: - Measuring

struct Measure: ViewModifier {
    let name: String
    func body(content: Content) -> some View {
        content.onGeometryChange(for: CGSize.self) { $0.size } action: { size in
            print("METRIC \(name) \(size.width) \(size.height)")
            fflush(stdout)
        }
    }
}

extension View {
    func measure(_ name: String) -> some View { modifier(Measure(name: name)) }
}

struct SafeAreaProbe: View {
    let name: String
    var body: some View {
        GeometryReader { proxy in
            Color.clear.onAppear {
                let i = proxy.safeAreaInsets
                print("METRIC \(name)-safe-area \(i.top) \(i.bottom) \(i.leading) \(i.trailing)")
                fflush(stdout)
            }
        }
    }
}

// MARK: - Colors and fonts

func components(_ c: PlatformColor) -> (CGFloat, CGFloat, CGFloat, CGFloat) {
    var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
    #if os(iOS)
    c.getRed(&r, green: &g, blue: &b, alpha: &a)
    #else
    if let s = c.usingColorSpace(.sRGB) { s.getRed(&r, green: &g, blue: &b, alpha: &a) }
    #endif
    return (r, g, b, a)
}

func dumpColor(_ name: String, _ color: PlatformColor) {
    #if os(iOS)
    for (label, style) in [("light", UIUserInterfaceStyle.light), ("dark", .dark)] {
        let resolved = color.resolvedColor(with: UITraitCollection(userInterfaceStyle: style))
        let (r, g, b, a) = components(resolved)
        print(String(format: "COLOR %@ %@ %.4f %.4f %.4f %.4f", name, label, r, g, b, a))
    }
    #else
    for (label, appearance) in [("light", NSAppearance.Name.aqua), ("dark", .darkAqua)] {
        NSAppearance(named: appearance)!.performAsCurrentDrawingAppearance {
            let (r, g, b, a) = components(color)
            print(String(format: "COLOR %@ %@ %.4f %.4f %.4f %.4f", name, label, r, g, b, a))
        }
    }
    #endif
}

func dumpColors() {
    #if os(iOS)
    let colors: [(String, UIColor)] = [
        ("systemBlue", .systemBlue), ("systemRed", .systemRed), ("systemOrange", .systemOrange),
        ("systemYellow", .systemYellow), ("systemGreen", .systemGreen), ("systemIndigo", .systemIndigo),
        ("systemPurple", .systemPurple), ("systemPink", .systemPink), ("systemTeal", .systemTeal),
        ("systemGray", .systemGray), ("systemGray2", .systemGray2), ("systemGray3", .systemGray3),
        ("systemGray4", .systemGray4), ("systemGray5", .systemGray5), ("systemGray6", .systemGray6),
        ("label", .label), ("secondaryLabel", .secondaryLabel), ("tertiaryLabel", .tertiaryLabel),
        ("quaternaryLabel", .quaternaryLabel), ("separator", .separator),
        ("opaqueSeparator", .opaqueSeparator), ("systemFill", .systemFill),
        ("secondarySystemFill", .secondarySystemFill), ("tertiarySystemFill", .tertiarySystemFill),
        ("quaternarySystemFill", .quaternarySystemFill), ("systemBackground", .systemBackground),
        ("secondarySystemBackground", .secondarySystemBackground),
        ("tertiarySystemBackground", .tertiarySystemBackground),
        ("systemGroupedBackground", .systemGroupedBackground),
        ("secondarySystemGroupedBackground", .secondarySystemGroupedBackground),
        ("tertiarySystemGroupedBackground", .tertiarySystemGroupedBackground),
        ("tintColor", .tintColor),
    ]
    #else
    let colors: [(String, NSColor)] = [
        ("systemBlue", .systemBlue), ("systemRed", .systemRed), ("systemOrange", .systemOrange),
        ("systemYellow", .systemYellow), ("systemGreen", .systemGreen), ("systemIndigo", .systemIndigo),
        ("systemGray", .systemGray), ("labelColor", .labelColor),
        ("secondaryLabelColor", .secondaryLabelColor), ("tertiaryLabelColor", .tertiaryLabelColor),
        ("quaternaryLabelColor", .quaternaryLabelColor), ("separatorColor", .separatorColor),
        ("gridColor", .gridColor), ("windowBackgroundColor", .windowBackgroundColor),
        ("controlBackgroundColor", .controlBackgroundColor),
        ("textBackgroundColor", .textBackgroundColor),
        ("underPageBackgroundColor", .underPageBackgroundColor), ("controlColor", .controlColor),
        ("controlAccentColor", .controlAccentColor),
        ("selectedContentBackgroundColor", .selectedContentBackgroundColor),
        ("unemphasizedSelectedContentBackgroundColor", .unemphasizedSelectedContentBackgroundColor),
        ("systemFill", .systemFill), ("secondarySystemFill", .secondarySystemFill),
        ("tertiarySystemFill", .tertiarySystemFill), ("quaternarySystemFill", .quaternarySystemFill),
        ("controlTextColor", .controlTextColor), ("placeholderTextColor", .placeholderTextColor),
    ]
    #endif
    for (name, color) in colors { dumpColor(name, color) }
}

func dumpFonts() {
    #if os(iOS)
    let styles: [(String, UIFont.TextStyle)] = [
        ("largeTitle", .largeTitle), ("title1", .title1), ("title2", .title2), ("title3", .title3),
        ("headline", .headline), ("body", .body), ("callout", .callout),
        ("subheadline", .subheadline), ("footnote", .footnote), ("caption1", .caption1),
        ("caption2", .caption2),
    ]
    for (name, style) in styles {
        let f = UIFont.preferredFont(forTextStyle: style)
        let traits = f.fontDescriptor.object(forKey: .traits) as? [UIFontDescriptor.TraitKey: Any]
        let weight = traits?[.weight] as? CGFloat ?? 0
        print(String(format: "FONT %@ %.2f %.2f %.3f %@", name, f.pointSize, f.lineHeight, weight, f.fontName))
    }
    #else
    let styles: [(String, NSFont.TextStyle)] = [
        ("largeTitle", .largeTitle), ("title1", .title1), ("title2", .title2), ("title3", .title3),
        ("headline", .headline), ("body", .body), ("callout", .callout),
        ("subheadline", .subheadline), ("footnote", .footnote), ("caption1", .caption1),
        ("caption2", .caption2),
    ]
    for (name, style) in styles {
        let f = NSFont.preferredFont(forTextStyle: style)
        let traits = f.fontDescriptor.object(forKey: .traits) as? [NSFontDescriptor.TraitKey: Any]
        let weight = traits?[.weight] as? CGFloat ?? 0
        let line = f.ascender - f.descender + f.leading
        print(String(format: "FONT %@ %.2f %.2f %.3f %@", name, f.pointSize, line, weight, f.fontName))
    }
    #endif
    fflush(stdout)
}

// MARK: - Pages

struct ControlsPage: View {
    @State private var on = true
    @State private var off = false
    @State private var value = 0.4
    @State private var text = "Ferris"
    @State private var segment = 0

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                Text("Buttons").font(.headline)
                HStack {
                    Button("Prominent") {}.buttonStyle(.borderedProminent).measure("button-prominent")
                    Button("Bordered") {}.buttonStyle(.bordered).measure("button-bordered")
                    Button("Plain") {}.buttonStyle(.borderless).measure("button-borderless")
                }
                HStack {
                    Button("Glass") {}.buttonStyle(.glass).measure("button-glass")
                    Button("Glass prominent") {}.buttonStyle(.glassProminent).measure("button-glass-prominent")
                    Button("Delete", role: .destructive) {}.buttonStyle(.borderedProminent).measure("button-destructive")
                }
                HStack {
                    Button("Large") {}.buttonStyle(.borderedProminent).controlSize(.large).measure("button-large")
                    Button("Small") {}.buttonStyle(.bordered).controlSize(.small).measure("button-small")
                    #if os(macOS)
                    Button("Push") {}.measure("button-push")
                    #endif
                }
                Text("Toggles").font(.headline)
                HStack(spacing: 20) {
                    Toggle("", isOn: $on).labelsHidden().toggleStyle(.switch).measure("switch")
                    Toggle("", isOn: $off).labelsHidden().toggleStyle(.switch)
                    #if os(macOS)
                    Toggle("Checkbox", isOn: $on).toggleStyle(.checkbox).measure("checkbox")
                    #endif
                }
                Slider(value: $value).frame(width: 260).measure("slider")
                ProgressView(value: 0.6).frame(width: 260).measure("progress-linear")
                HStack(spacing: 20) {
                    ProgressView().measure("progress-spinner")
                    ProgressView().controlSize(.large).measure("progress-spinner-large")
                    Gauge(value: 0.6) {}.gaugeStyle(.accessoryCircularCapacity).measure("gauge")
                }
                TextField("Your name", text: $text).textFieldStyle(.roundedBorder).frame(width: 260).measure("textfield-rounded")
                TextField("Plain", text: $text).frame(width: 260).measure("textfield-plain")
                Picker("", selection: $segment) {
                    Text("One").tag(0); Text("Two").tag(1); Text("Three").tag(2)
                }.pickerStyle(.segmented).frame(width: 260).measure("segmented")
                Stepper("Stepper", value: $value).frame(width: 260).measure("stepper")
                HStack {
                    Text("Badge")
                    Text("3").font(.caption.bold()).foregroundStyle(.white).padding(.horizontal, 6)
                        .background(Capsule().fill(.red)).measure("badge")
                }
                RoundedRectangle(cornerRadius: 26).fill(.background.secondary).frame(height: 60)
            }
            .padding()
        }
        .measure("controls-page")
    }
}

struct ListPage: View {
    @State private var on = true
    var body: some View {
        #if os(iOS)
        NavigationStack {
            List {
                Section("Section") {
                    Text("Row").measure("list-row")
                    NavigationLink("Chevron row") { Text("Detail") }.measure("list-row-chevron")
                    Toggle("Toggle row", isOn: $on)
                    LabeledContent("Detail", value: "Value")
                }
                Section {
                    Label("Icon row", systemImage: "star").measure("list-row-icon")
                } footer: { Text("Footer text") }
            }
            .listStyle(.insetGrouped)
            .navigationTitle("Large title")
            .toolbar { ToolbarItem { Button("Edit") {} } }
            .background(SafeAreaProbe(name: "navigation"))
        }
        #else
        NavigationSplitView {
            List {
                Label("Home", systemImage: "house").measure("sidebar-row")
                Label("Controls", systemImage: "slider.horizontal.3")
                Label("Counter", systemImage: "number")
            }
            .navigationSplitViewColumnWidth(220)
        } detail: {
            Form {
                Section("Section") {
                    Text("Row").measure("form-row")
                    Toggle("Toggle row", isOn: $on)
                    LabeledContent("Detail", value: "Value")
                }
            }
            .formStyle(.grouped)
            .navigationTitle("Wings")
            .toolbar { ToolbarItem { Button("Edit", systemImage: "pencil") {} } }
            .background(SafeAreaProbe(name: "detail"))
        }
        #endif
    }
}

struct TabsPage: View {
    var body: some View {
        TabView {
            Tab("Home", systemImage: "house") {
                Color.clear.background(SafeAreaProbe(name: "tab"))
            }
            Tab("Controls", systemImage: "slider.horizontal.3") { Text("Controls") }
            Tab("Counter", systemImage: "number") { Text("Counter") }
            Tab("Notes", systemImage: "note.text") { Text("Notes") }
        }
    }
}

struct AlertPage: View {
    @State private var show = false
    var body: some View {
        Text("Alert")
            .onAppear { DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { show = true } }
            .alert("Discard draft?", isPresented: $show) {
                Button("Cancel", role: .cancel) {}
                Button("Discard", role: .destructive) {}
            } message: {
                Text("Your changes will be lost.")
            }
    }
}

func page() -> String {
    CommandLine.arguments.dropFirst().first { !$0.hasPrefix("-") } ?? "controls"
}

@ViewBuilder
func pageView() -> some View {
    switch page() {
    case "list": ListPage()
    case "tabs": TabsPage()
    case "alert": AlertPage()
    default: ControlsPage()
    }
}

#if os(macOS)
// AppKit bootstrap: a plain NSWindow hosting the SwiftUI page, so the window always opens
// even when the binary is launched from a script.
final class AppDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow?

    func applicationDidFinishLaunching(_ notification: Notification) {
        dumpColors()
        dumpFonts()
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 760, height: 720),
            styleMask: [.titled, .closable, .resizable, .miniaturizable, .fullSizeContentView],
            backing: .buffered, defer: false)
        window.title = "Wings reference"
        window.contentView = NSHostingView(rootView: pageView())
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.window = window
        NSApp.activate()
    }
}

@main
enum ReferenceMain {
    static func main() {
        let app = NSApplication.shared
        let delegate = AppDelegate()
        app.delegate = delegate
        app.setActivationPolicy(.regular)
        app.run()
    }
}
#else
@main
struct ReferenceApp: App {
    init() {
        dumpColors()
        dumpFonts()
    }

    var body: some Scene {
        WindowGroup { pageView() }
    }
}
#endif
