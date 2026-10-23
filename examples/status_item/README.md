# Native status item example

A small macOS capability probe for `gpui_kit::status_item`. The menu bar contains
only a template image. Clicking it opens a `WindowKind::PopUp` with generated
microphone/system meters, elapsed demo time, and controls sharing the main
window's counter and demo state. Nothing records audio or changes login settings.

From the workspace root:

```sh
cargo run --locked -p status_item
```

For a named, ad hoc signed development bundle that remains under `target/`:

```sh
./examples/status_item/run-macos.sh
# Build without launching:
./examples/status_item/run-macos.sh --build-only
```

The script honors `CARGO_TARGET_DIR`. It never installs the app or uses a signing
identity/keychain. Quit an existing example before launching a rebuilt bundle.
On other platforms, the ordinary window shows the unsupported status-item error;
no native status item is created.

The main window also shows a launch-reason snapshot captured synchronously in
the launch callback and a read-only login-item status query. `Unknown` launch
reason is retained as unknown; it is not inferred to be an explicit launch.
No example code calls registration, unregistration, or System Settings.

## Behavior to verify on macOS

- Click the native waveform item while another application is frontmost. The
  popup should receive keyboard focus without raising the example's main window.
- Start the demo; both labeled meters move and elapsed time advances inside the
  popup. The native image gains a dot, and its tooltip/accessibility name changes.
  Stop the demo and verify both views show idle and the meters return to zero.
- Increment the counter through either surface. Reopen the popup and verify both
  views show the same count; no second session is created.
- Press Tab/Shift-Tab and activate buttons using the keyboard. Escape, an outside
  click, or a second click on the native item should dismiss the popup. The
  native item's selected highlight should last only while its popup is open.
- Choose **Open main window**. It dismisses the popup and activates the existing
  main window. Reopening does not create duplicate windows.
- Repeat open/dismiss rapidly, including clicking and holding the native item
  for the second activation. Watch for a close-then-reopen ordering bug.
- Test with light/dark appearance, a crowded/auto-hidden menu bar, full-screen
  apps, a notched display, and multiple displays with different scales. Native
  geometry is read on each opening. While open, a missing or changed anchor
  dismisses the popup so its next opening uses the new display coordinates.
- Inspect with VoiceOver/Accessibility Inspector: native button name, popup
  buttons, each meter's label/value, keyboard focus, and selected native state.
  The stable demo-status node uses AccessKit's polite live-region property for
  Start/Stop changes. Elapsed time, counter and meter values are outside that
  node; repeated meter samples must not announce the demo status. Actual
  VoiceOver speech still needs to be checked in the native app.
- Close and reopen the popup, then quit. Its recurring sampling task belongs to
  the popup entity; closing the popup cancels it. Native callbacks retain only a
  weak session reference, and dropping the session removes the status item.

## Current boundary

Closing the main window quits this prototype. The pinned GPUI API does not yet
expose the per-window hide/show and native reopen contract required for a
background app. This example does not simulate those capabilities by hiding the
entire application or bypassing GPUI with native window calls. Login-item
registration is intentionally absent.

Every window is opened through `gpui_kit::open_window`, which supplies its single
`Root`. The main view retains the session; the session retains the native item
and only a window handle for the popup. The popup and native callback hold weak
session references. Popup release uses a generation check so an old release
cannot clear a newer popup's native highlight.

Window opening, closing and activation are deferred until the session update
has returned. GPUI can render a new window synchronously inside `open_window`;
holding the session's mutable lease through that call would panic when the
popup reads the same session. The regression smoke path is **Open popup** from
the main window, **Open main window** from the popup, then rapid native-item
open/close. A reserved generation rejects a cancelled opening and prevents an
older release from changing a newer popup.

The two original PNG fixtures are monochrome template images at three physical
pixels per 18-point logical icon. Regenerate them with:

```sh
python3 examples/status_item/fixtures/generate_icons.py
```
