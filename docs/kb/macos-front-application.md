# The front application on macOS

Last update: 2026-11-14

[Decision 0035](../decisions/0035-front-application-selects-the-profile.md)
uses these facts. The shell observes the front application, and the engine
selects the profile.

## Notices and calls

| Fact | Confidence | Source |
|---|---|---|
| `NSWorkspace` posts `NSWorkspaceDidActivateApplicationNotification` when an application becomes active. The notice goes to the notification centre of `NSWorkspace` (`NSWorkspace.sharedWorkspace.notificationCenter`), not to the default notification centre. | Source | Apple documentation for `NSWorkspace` |
| The `userInfo` of the notice has the key `NSWorkspaceApplicationKey`. Its value is the `NSRunningApplication` of the application that became active. | Source | Apple documentation for `NSWorkspace` |
| `NSWorkspace.frontmostApplication` gives the application that receives the key events now. | Source | Apple documentation for `NSWorkspace` |
| `NSRunningApplication.bundleIdentifier` is `nil` for an application that has no `Info.plist`. `processIdentifier` and `localizedName` are also available. | Source | Apple documentation for `NSRunningApplication` |
| A bundle identifier contains only ASCII letters, digits, `-`, and `.`. | Source | Apple documentation for `CFBundleIdentifier` |
| The bundle identifier is in the `Info.plist` file in the bundle. A move of the bundle to a different folder does not change it. The path of the bundle changes. | Inference | The two facts above |
| An observer of the notification centre of `NSWorkspace` gets its notices on the main thread of the application, with no timer and no thread of its own. OpenCraft observes the power notices in this way (`crates/app/src/power.rs`). | Source | The code of `power.rs`; [decision 0034](../decisions/0034-device-dark-state.md) |
| OpenCraft observes `NSWorkspaceDidActivateApplicationNotification` from the start of the process until the process ends, also when no window is open. For each notice, and one time for the front application at the start, the shell sends `Command::FrontApplication`. The shell does not compare applications, and it does not read the setting. | Source | `crates/app/src/front_app.rs`; [decision 0035](../decisions/0035-front-application-selects-the-profile.md) |

## Not known

- Does macOS make `loginwindow` or the screen saver the active application
  at a screen lock or at the start of the screen saver? If yes, does that
  notice come before or after the lock notice? The engine skips a switch
  while the screen is locked or the computer sleeps
  ([decision 0035](../decisions/0035-front-application-selects-the-profile.md),
  section 8). If the activation notice arrives before the lock notice, the
  profile can still change. The order is not known, and the code does not
  add a filter for those bundle identifiers.
- Does the application that was active before a lock get a new activation
  notice after the unlock?
- Does macOS compare two bundle identifiers with no difference between
  small and capital letters? Decision 0035 compares them in that way.
- The time from the activation of an application to the notice in
  OpenCraft. Task [#341](https://github.com/shantanugoel/OpenCraft/issues/341)
  measures the time to the new profile on the device.

## Sources

- Apple, `NSWorkspace`:
  <https://developer.apple.com/documentation/appkit/nsworkspace>
- Apple, `NSRunningApplication`:
  <https://developer.apple.com/documentation/appkit/nsrunningapplication>
- Apple, `CFBundleIdentifier`:
  <https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleidentifier>
