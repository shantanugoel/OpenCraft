# Open and quit applications, files, and URLs

Last update: 2026-11-14

The implementation is in `crates/actions/src/system/macos.rs`. The actions
"Open an application", "Open a URL", and "Quit an application" use public
calls. None of them needs a new permission prompt.

## Open an application, a file, or a folder

| Fact | Confidence | Source |
|---|---|---|
| `/usr/bin/open -a <name or path>` opens an application. If the application runs, it comes to the front. | Source | `open(1)` |
| `/usr/bin/open -- <path>` opens a file or a folder in its default application. A folder opens in Finder. | Measured | One call with a folder; exit code 0 |
| `open -a <app> --args <words>` gives the words to the application as its arguments when the system starts the application. With Calculator not running, the process list showed the words after the executable path. | Measured | One run on macOS 27.2; process list |
| A second `open -a <app> --args <other words>` for a running application does not start it again, and does not change its arguments. | Measured | The same run; the process list did not change |
| `open -a` with a path that does not exist, or a name that no application has, ends with exit code 1. The text of the error goes to the standard error output. | Measured | One run each |
| The program `open` does not expand `~`. OpenCraft replaces a leading `~/` with the home folder before the call. | Measured | Source of the code: `expand_home` |
| `open -a <app> -- <url>` opens the URL in that application. With no `-a`, the system uses the default application for the URL. | Source | `open(1)` |
| `open` sends no Apple event, so macOS asks for no Automation permission. | Inference | [macos-permissions.md](macos-permissions.md); no prompt in the runs above |

## Quit an application

| Fact | Confidence | Source |
|---|---|---|
| `NSWorkspace.runningApplications` gives the running applications. `NSRunningApplication.terminate` asks one to quit, as the Quit menu item does. `forceTerminate` ends it at once, with no chance to save. Both are public AppKit calls and send no Apple event. | Source | AppKit headers; Apple documentation for `NSRunningApplication` |
| OpenCraft makes no use of AppleScript or System Events for this action. Thus it needs no Automation permission and no usage text for Apple events. | Inference | The calls above; [macos-permissions.md](macos-permissions.md) |
| With Calculator running, the call `terminate` ended it, with exit code 0 and no other effect. The call `forceTerminate` with the bundle identifier also ended it. A call for an application that does not run found no application and OpenCraft gave the error "the application does not run". | Measured | One run each on macOS 27.2, with the example `system` of `opencraft-actions` and an application that the test started |
| `terminate` returns at once. It does not wait for the application to end. An application can show a dialog ("Save changes?"), or can refuse the request, and then it keeps running. | Source | Apple documentation for `NSRunningApplication` |
| OpenCraft finds an application by its name, its bundle identifier, or the path of its bundle, and ignores the case of letters. It uses only applications with a Dock icon or a menu bar item, and never its own process. All processes that match get the request. | Inference | The code of `macos_quit.rs` and its tests |

## Not known

- How an application that is already running uses new arguments. The system
  does not give them. An application can have its own way to get them, for
  example a URL scheme or a command-line helper.

## Sources

- macOS manual page `open(1)`.
