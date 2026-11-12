# Security policy

## Report a vulnerability

Do not report a vulnerability in a public issue.

Use the private vulnerability reporting of GitHub for this repository:

1. Open the "Security" tab of the repository.
2. Click "Report a vulnerability".
3. Describe the problem, the version of OpenCraft, and the steps to repeat it.

Remove personal data from your report. Examples are the device serial
number, a home path, and a host name.

The owner reads each report and answers in it. OpenCraft is a project of one
person. Thus the time to an answer can be some days.

## Supported versions

Only the latest release gets a correction.

## What is in scope

OpenCraft runs commands that the user sets, and it sends keys. This is by
design. These behaviors are not vulnerabilities:

- The action "run a shell command" runs the command that the user wrote in
  the profile. The command has the rights of the user.
- The actions "send a hotkey", "type text", and "media keys" send key events
  to other applications.
- A person who can write the profile files of the user can make OpenCraft run
  commands. Protect the profiles directory with the file permissions of the
  user account.

These problems are in scope:

- A profile, an image, or a device input that makes OpenCraft run a command
  that the user did not set.
- A memory safety fault that a file or a device report can cause.
- A file path in a profile that makes OpenCraft read or write outside the
  profiles directory.
- A secret or personal data that OpenCraft writes to the log file.

These problems are not in scope:

- A fault in a dependency. Report it to the maintainer of the dependency.
- A fault in the vendor software or in the firmware of the device.
- A fault that needs a malicious device or physical access to the computer of
  the user.
