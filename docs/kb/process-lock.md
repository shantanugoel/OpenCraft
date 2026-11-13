# The process lock and child processes

The application locks the file `opencraft.lock` with `flock`
([decisions 0019 and 0028](../decisions/0028-second-start-opens-the-window.md)).
This entry holds what we found when the test of the lock failed one time in
a full test run
([issue 223](https://github.com/shantanugoel/OpenCraft/issues/223)).

## Facts (2026-11-13)

**Measured.** The computer is the one of
[Release measurements](gpui.md#release-measurements). The test programs are
small programs that use only the standard library and `flock`. They are not in
the repository.

| Fact | Confidence | Source |
|---|---|---|
| A lock from `flock` belongs to the open file, not to a file descriptor. It stays until the last descriptor of that open file closes, also the descriptors of other processes. | Source | `man 2 flock` |
| A process that starts has a copy of each open file descriptor of the parent, until it runs `exec`. `exec` closes the descriptors with `O_CLOEXEC`. The standard library opens each file with `O_CLOEXEC`. A child that runs holds no copy of the lock file. | Source; Measured | `crates/app/src/lock.rs`, `acquire`; `a_running_child_process_does_not_keep_the_lock` |
| One thread started child processes in a loop, and the main thread locked a file, closed it, and locked it again. Without the other thread, 0 of 20 000 second locks failed. With it, 3772 and 4414 of 20 000 failed. A thread that starts a process can thus make the lock of a different thread fail for a short time after the lock file closed. | Measured | Test program with one thread for the children |
| The unit test `a_second_lock_fails_until_the_first_file_closes` failed in 30 of 30 runs when its sequence ran 2000 times, and the test `only_a_different_opencraft_process_is_a_holder` started 300 processes at the same time. With a lock that makes the tests of the lock run one at a time, 100 of 100 runs passed with the same load. | Measured | `cargo test -p opencraft-app lock::` with a temporary change |
| In one thread, with no other thread, a loop of 1000 rounds (lock, start `sleep 5`, close the file, lock again) had the second lock fail 7 times when the program was started by name (`Command::new("sleep")`), and 58 times with 10 busy processes in the background. With the full path (`/bin/sleep`), it failed 0 times, also with the busy processes. | Measured | Test program in one thread |
| The reason for the difference between a name and a full path: `posix_spawnp` of macOS tries each directory of `PATH`. Each failed try seems to leave a short child process that has the copies of the descriptors until the system removes it. We did not see the process, because it is gone at the next look: `lsof` and a scan of `F_GETPATH` in the failing test found nothing. | Inference | The two measurements above |
| The lock code is correct. It keeps the file until the process stops, and the shell command action starts `/bin/sh` with a full path. A child that is in its start when OpenCraft stops can keep the lock for a short time. A second start in that time sees a lock and asks the holder, which does not exist any more. It then writes its message. We did not measure this case. | Inference | `crates/app/src/main.rs`, `crates/actions/src/system.rs` |

## Rule for tests

A test that locks a file and a test that starts a process must not run at the
same time in one process. The tests of the lock take one `Mutex`, and they
start a program with its full path.
