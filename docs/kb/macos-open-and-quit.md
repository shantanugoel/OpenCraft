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
| `open` sends no Apple event, so macOS asks for no Automation permission. | Inference | [macos-permissions.md](macos-permissions.md); no prompt in the runs above |

## Not known

- How an application that is already running uses new arguments. The system
  does not give them. An application can have its own way to get them, for
  example a URL scheme or a command-line helper.

## Sources

- macOS manual page `open(1)`.
