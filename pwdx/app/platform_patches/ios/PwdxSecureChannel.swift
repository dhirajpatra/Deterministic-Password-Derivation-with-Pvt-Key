import Flutter
import UIKit

/// Hardened clipboard for pwdx: local-only (no Universal Clipboard sync) and self-expiring.
/// Register it once the Flutter engine exists, e.g. in AppDelegate:
///     PwdxSecureChannel.register(with: engineBridge.applicationRegistrar.messenger())   // newer template
///     PwdxSecureChannel.register(with: controller.binaryMessenger)                      // classic template
/// Without this file the app still works; Dart falls back to the normal clipboard.
enum PwdxSecureChannel {
    static func register(with messenger: FlutterBinaryMessenger) {
        let channel = FlutterMethodChannel(name: "pwdx/secure", binaryMessenger: messenger)
        channel.setMethodCallHandler { call, result in
            switch call.method {
            case "copySensitive":
                guard let args = call.arguments as? [String: Any], let text = args["text"] as? String else {
                    result(FlutterError(code: "bad_args", message: "text missing", details: nil))
                    return
                }
                UIPasteboard.general.setItems(
                    [["public.utf8-plain-text": text]],
                    options: [.localOnly: true, .expirationDate: Date().addingTimeInterval(30)]
                )
                result(nil)
            case "clearClipboard":
                UIPasteboard.general.items = []
                result(nil)
            default:
                result(FlutterMethodNotImplemented)
            }
        }
    }
}
