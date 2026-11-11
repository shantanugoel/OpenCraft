#!/usr/bin/env bash
# Measure the memory and the processor use of ALL processes of one application.
# The procedure for the owner is in scripts/README.md. The issue is #20.
#
# Usage:
#   scripts/measure-footprint.sh <app name | bundle path> [options]
#   scripts/measure-footprint.sh --pid <pid> [options]
#
# Options:
#   --interval <seconds>  time between two samples (default 5)
#   --duration <seconds>  total time of the measurement (default 300)
#   --out <file.csv>      CSV output file (default footprint-<date>-<time>.csv)
#   --startup             start the application with "open -a" and print the time
#                         until the process set and its total footprint are stable
#                         (needs an application, not --pid; the application must
#                         not run yet)
#
# Needs: macOS, and only tools that macOS ships. Works with bash 3.2.
#
# HOW THE SCRIPT FINDS THE PROCESSES
#   On EACH sample the script reads the table of all processes (ps -axo
#   pid,ppid,comm). The process set is:
#     1. each process whose executable path is inside the bundle
#        (or, with --pid, the process with that PID), and
#     2. every descendant of those processes (the parent PID chain).
#   The set is found again on each sample, because processes start and stop.
#   Descendants are necessary, because a plug-in process can be a Node.js
#   process whose executable is outside the bundle. The script and its own
#   child processes (ps, footprint) are never in the set.
#
# HOW THE SCRIPT MEASURES
#   Memory:    "footprint" gives the physical footprint of a process. It is the
#              same number as "Memory" in Activity Monitor. "ps -o rss" gives the
#              resident size, which is a second, different number. The script
#              records both. All sizes are in KiB (1024 bytes). The sum of the
#              footprints of the processes is the total of the application.
#              "footprint" must run as root for a process of a different user.
#              In that case the field footprint_kb stays empty, and the summary
#              says how many values are missing.
#   Processor: the script does NOT use the %cpu column of ps. That value is a
#              decayed average over the life of the process, and it lags behind
#              a sudden change. The script reads the accumulated processor time
#              (ps -o cputime, user + system) on each sample, and divides the
#              difference between two samples by the real time between them.
#              100 percent is one full core, as in Activity Monitor and top.
#              The first sample has no processor value (no earlier sample). A
#              process that starts between two samples counts from zero. A
#              process that stops between two samples gives no value for the
#              last interval, so the total can be slightly low.
#
# CSV columns: sample, time, elapsed_s, pid, ppid, process, rss_kb,
#   footprint_kb, cpu_time_s, cpu_percent. One row for each process and each
#   sample. After the process rows, one row with pid "TOTAL" for the sample;
#   its "process" field says how many processes are in the sample.
#   The script replaces the characters comma and quote in a process name
#   with an underscore, so the file needs no quoting.

set -u

PROG="$(basename "$0")"
INTERVAL=5
DURATION=300
OUT=""
STARTUP=0
TARGET=""
ROOT_PID=""
BUNDLE=""
PARALLEL=8
STARTUP_POLL=1
STARTUP_STABLE_POLLS=5
STARTUP_TIMEOUT=120

usage() {
    cat <<EOF
Usage: $PROG <app name | bundle path> [options]
       $PROG --pid <pid> [options]

  --interval <seconds>  time between two samples (default $INTERVAL)
  --duration <seconds>  total time of the measurement (default $DURATION)
  --out <file.csv>      CSV output file (default footprint-<date>-<time>.csv)
  --startup             start the application and print the time until the
                        process set and its total footprint are stable
  -h, --help            show this text

Examples:
  $PROG "VSD Craft"
  $PROG "/Applications/VSD Craft.app" --interval 5 --duration 300 --out idle.csv
  $PROG --pid 1234 --out opencraft.csv
EOF
}

die() {
    echo "$PROG: $*" >&2
    exit 1
}

is_positive_number() {
    awk -v x="$1" 'BEGIN { exit !(x ~ /^[0-9]+(\.[0-9]+)?$/ && x + 0 > 0) }'
}

# --- arguments ---------------------------------------------------------------

while [ $# -gt 0 ]; do
    case "$1" in
        --interval | --duration | --out | --pid)
            [ $# -ge 2 ] || { usage >&2; exit 2; }
            case "$1" in
                --interval) INTERVAL="$2" ;;
                --duration) DURATION="$2" ;;
                --out) OUT="$2" ;;
                --pid) ROOT_PID="$2" ;;
            esac
            shift 2
            ;;
        --startup)
            STARTUP=1
            shift
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        -*)
            echo "$PROG: unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
        *)
            [ -z "$TARGET" ] || { echo "$PROG: only one application is allowed" >&2; exit 2; }
            TARGET="$1"
            shift
            ;;
    esac
done

is_positive_number "$INTERVAL" || die "--interval must be a number greater than 0"
is_positive_number "$DURATION" || die "--duration must be a number greater than 0"
if [ -n "$ROOT_PID" ]; then
    case "$ROOT_PID" in
        *[!0-9]* | "") die "--pid must be a number" ;;
    esac
    [ -z "$TARGET" ] || die "give an application or --pid, not both"
    [ "$STARTUP" -eq 0 ] || die "--startup needs an application, not --pid"
    ps -p "$ROOT_PID" >/dev/null 2>&1 || die "no process with PID $ROOT_PID"
elif [ -z "$TARGET" ]; then
    usage >&2
    exit 2
fi

# --- helpers -----------------------------------------------------------------

# Time in seconds with fractions. Perl ships with macOS. If it fails, use whole
# seconds.
if perl -MTime::HiRes -e 1 2>/dev/null; then
    now() { perl -MTime::HiRes=time -e 'printf "%.3f\n", time'; }
else
    now() { date +%s; }
fi

calc() {
    awk "BEGIN { printf \"%.3f\n\", $1 }"
}

find_bundle() {
    local name="$1" dir cand
    if [ -d "$name" ]; then
        (cd "$name" && pwd -P)
        return 0
    fi
    name="${name%.app}"
    for dir in /Applications "$HOME/Applications" /Applications/Utilities \
        /System/Applications /System/Applications/Utilities \
        /System/Library/CoreServices; do
        cand="$dir/$name.app"
        if [ -d "$cand" ]; then
            echo "$cand"
            return 0
        fi
    done
    cand="$(mdfind "kMDItemContentType == 'com.apple.application-bundle' && kMDItemFSName == '$name.app'" 2>/dev/null | head -n 1)"
    if [ -n "$cand" ] && [ -d "$cand" ]; then
        echo "$cand"
        return 0
    fi
    return 1
}

TMP="$(mktemp -d "${TMPDIR:-/tmp}/measure-footprint.XXXXXX")" || die "cannot make a temporary directory"
SNAP="$TMP/snap"
SEL="$TMP/sel"
FPF="$TMP/fp"
PREV="$TMP/prev"
PREVNEW="$TMP/prev.new"
DATA="$TMP/data.tsv"
: >"$PREV"
: >"$DATA"

cleanup() {
    rm -rf "$TMP"
}

# Write the process table to $SNAP as: pid ppid rss_kb cpu_seconds path
snapshot() {
    ps -axo pid=,ppid=,rss=,cputime=,comm= | awk '
        function secs(t,   d, n, p, s) {
            # cputime has the form [d-][h:]m:s.f ; macOS prints m:s.f
            d = 0
            if (t ~ /-/) { split(t, a, "-"); d = a[1]; t = a[2] }
            n = split(t, p, ":")
            s = 0
            for (i = 1; i <= n; i++) s = s * 60 + p[i]
            return d * 86400 + s
        }
        {
            line = $0
            sub(/^ +/, "", line)
            pid = $1; ppid = $2; rss = $3; cpu = secs($4)
            for (i = 0; i < 4; i++) sub(/^[^ ]+ +/, "", line)
            printf "%s %s %s %.2f %s\n", pid, ppid, rss, cpu, line
        }' >"$SNAP"
}

# Write the selected rows of $SNAP to $SEL. $1 is "bundle" or "pid".
# Roots: the processes inside the bundle (or the given PID). Then every
# descendant. The script itself and its descendants are removed.
select_processes() {
    awk -v mode="$1" -v bundle="$BUNDLE" -v rootpid="$ROOT_PID" -v self="$$" '
        {
            pid = $1
            ppid_of[pid] = $2
            line[pid] = $0
            order[++n] = pid
            path = $0
            for (i = 0; i < 4; i++) sub(/^[^ ]+ +/, "", path)
            if (mode == "pid") {
                if (pid == rootpid) inset[pid] = 1
            } else if (index(path, bundle "/") == 1) {
                inset[pid] = 1
            }
        }
        END {
            skip[self] = 1
            do {
                changed = 0
                for (k = 1; k <= n; k++) {
                    p = order[k]
                    if (!(p in inset) && (ppid_of[p] in inset)) { inset[p] = 1; changed = 1 }
                    if (!(p in skip) && (ppid_of[p] in skip)) { skip[p] = 1; changed = 1 }
                }
            } while (changed)
            for (k = 1; k <= n; k++) {
                p = order[k]
                if ((p in inset) && !(p in skip)) print line[p]
            }
        }' "$SNAP" >"$SEL"
}

select_now() {
    if [ -n "$ROOT_PID" ]; then
        select_processes pid
    else
        select_processes bundle
    fi
}

# Print the physical footprint of one process in KiB. Print nothing on failure.
fp_kb() {
    footprint -f bytes "$1" 2>/dev/null | awk '
        /Footprint:/ {
            for (i = 1; i < NF; i++)
                if ($i == "Footprint:") { printf "%d\n", $(i + 1) / 1024; exit }
        }'
}

# Read the footprint of each PID in $SEL. Write "pid kb" lines to $FPF.
# footprint needs about 0.1 second for each process, so run a few at a time.
collect_footprints() {
    local pid rest count=0
    rm -f "$TMP"/fp.*
    while read -r pid rest; do
        (fp_kb "$pid" >"$TMP/fp.$pid") &
        count=$((count + 1))
        if [ $((count % PARALLEL)) -eq 0 ]; then
            wait
        fi
    done <"$SEL"
    wait
    : >"$FPF"
    while read -r pid rest; do
        if [ -s "$TMP/fp.$pid" ]; then
            echo "$pid $(cat "$TMP/fp.$pid")" >>"$FPF"
        fi
    done <"$SEL"
}

# --- summary -----------------------------------------------------------------

summary() {
    [ -s "$DATA" ] || { echo "No data was collected."; return; }
    awk -F'\t' '
        function sorted(arr, n,   i, j, t) {
            for (i = 2; i <= n; i++) {
                t = arr[i]
                for (j = i - 1; j >= 1 && arr[j] > t; j--) arr[j + 1] = arr[j]
                arr[j + 1] = t
            }
        }
        function mib(kb) { return sprintf("%.1f MiB", kb / 1024) }
        $1 == "T" {
            ns++
            cnt[ns] = $3; rss[ns] = $4; fpt[ns] = $5
            if ($6 != "") { nc++; cpu[nc] = $6 }
            miss += $7
            next
        }
        $1 == "P" {
            key = $3 " " $4
            if (!(key in seen)) { seen[key] = ++np; kpid[np] = $3; kname[np] = $4 }
            i = seen[key]
            present[i]++
            if ($6 != "" && $6 + 0 > maxfp[i]) maxfp[i] = $6 + 0
            if ($5 != "" && $5 + 0 > maxrss[i]) maxrss[i] = $5 + 0
        }
        END {
            printf "Samples: %d", ns
            if (nc == 0) printf "  (too few for a processor value; use at least two)"
            printf "\n"
            for (i = 1; i <= ns; i++) { c[i] = cnt[i]; r[i] = rss[i]; f[i] = fpt[i] }
            sorted(c, ns); sorted(r, ns); sorted(f, ns)
            med = int((ns + 1) / 2)
            if (ns % 2 == 0) {
                fmed = (f[ns / 2] + f[ns / 2 + 1]) / 2
                rmed = (r[ns / 2] + r[ns / 2 + 1]) / 2
            } else { fmed = f[med]; rmed = r[med] }
            printf "Processes:           min %d, max %d\n", c[1], c[ns]
            printf "Total footprint:     min %s, median %s, max %s\n", mib(f[1]), mib(fmed), mib(f[ns])
            printf "Total resident size: min %s, median %s, max %s\n", mib(r[1]), mib(rmed), mib(r[ns])
            if (nc > 0) {
                sum = 0; mx = 0
                for (i = 1; i <= nc; i++) { sum += cpu[i]; if (cpu[i] > mx) mx = cpu[i] }
                printf "Total processor use: mean %.1f%%, max %.1f%% (100%% = one core)\n", sum / nc, mx
            } else {
                printf "Total processor use: no value\n"
            }
            if (miss > 0)
                printf "Warning: %d process values have no footprint (a process of a different user, or a process that stopped). The footprint totals are too low.\n", miss
            printf "Largest 5 processes by footprint (maximum over all samples):\n"
            printf "  %-8s %-32s %14s %14s %8s\n", "PID", "Name", "Footprint", "Resident", "Samples"
            for (k = 1; k <= 5; k++) {
                best = 0; bi = 0
                for (i = 1; i <= np; i++)
                    if (!(i in used) && (bi == 0 || maxfp[i] > best)) { best = maxfp[i]; bi = i }
                if (bi == 0) break
                used[bi] = 1
                printf "  %-8s %-32s %14s %14s %8d\n", kpid[bi], substr(kname[bi], 1, 32), mib(maxfp[bi]), mib(maxrss[bi]), present[bi]
            }
        }' "$DATA"
}

on_interrupt() {
    echo
    echo "Interrupted. Summary of the samples so far:"
    summary
    cleanup
    exit 130
}

trap cleanup EXIT
trap on_interrupt INT TERM

# --- find the application ----------------------------------------------------

if [ -z "$ROOT_PID" ]; then
    BUNDLE="$(find_bundle "$TARGET")" || die "cannot find an application named \"$TARGET\""
    BUNDLE="${BUNDLE%/}"
fi

# --- startup mode ------------------------------------------------------------

run_startup() {
    local t0 t first="" prev_n=-1 prev_total=0 prev_t="" stable=0 win_start="" n total
    snapshot
    select_now
    if [ -s "$SEL" ]; then
        die "$BUNDLE already runs. Quit it first, and run --startup again."
    fi
    echo "Starting $(basename "$BUNDLE") with open -a. The stability check polls every ${STARTUP_POLL} s."
    t0="$(now)"
    open -a "$BUNDLE" || die "open -a failed"
    while :; do
        t="$(calc "$(now) - $t0")"
        snapshot
        select_now
        n="$(wc -l <"$SEL" | tr -d ' ')"
        total=0
        if [ "$n" -gt 0 ]; then
            [ -n "$first" ] || first="$t"
            collect_footprints
            total="$(awk '{ s += $2 } END { print s + 0 }' "$FPF")"
        fi
        # Stable: same number of processes, and the total footprint changes by
        # 1 percent or less, for STARTUP_STABLE_POLLS polls in a row.
        if [ "$n" -gt 0 ] && [ "$n" -eq "$prev_n" ] && awk -v a="$total" -v b="$prev_total" \
            'BEGIN { d = a - b; if (d < 0) d = -d; exit !(b > 0 && d <= b * 0.01) }'; then
            [ "$stable" -gt 0 ] || win_start="$prev_t"
            stable=$((stable + 1))
        else
            stable=0
        fi
        if [ "$stable" -ge "$STARTUP_STABLE_POLLS" ]; then
            echo "First process seen after:       ${first} s"
            echo "Process set and footprint stable from: ${win_start} s after the start"
            echo "At that time: $n processes, total footprint $(awk -v k="$total" 'BEGIN { printf "%.1f MiB", k / 1024 }')"
            echo "This is NOT the time to a ready device. A person measures that with a stopwatch."
            return 0
        fi
        prev_n="$n"
        prev_total="$total"
        prev_t="$t"
        if awk -v t="$t" -v m="$STARTUP_TIMEOUT" 'BEGIN { exit !(t >= m) }'; then
            echo "The process set was not stable after $STARTUP_TIMEOUT s."
            echo "Last sample: $n processes, total footprint $(awk -v k="$total" 'BEGIN { printf "%.1f MiB", k / 1024 }')"
            return 1
        fi
        sleep "$STARTUP_POLL"
    done
}

if [ "$STARTUP" -eq 1 ]; then
    run_startup
    exit $?
fi

# --- measurement loop --------------------------------------------------------

if [ -z "$OUT" ]; then
    OUT="footprint-$(date +%Y%m%d-%H%M%S).csv"
fi
echo "sample,time,elapsed_s,pid,ppid,process,rss_kb,footprint_kb,cpu_time_s,cpu_percent" >"$OUT" ||
    die "cannot write $OUT"

if [ -n "$ROOT_PID" ]; then
    echo "Measuring PID $ROOT_PID and its descendants."
else
    echo "Measuring all processes of $BUNDLE"
fi
echo "Interval ${INTERVAL} s, duration ${DURATION} s, output $OUT"

START="$(now)"
PREV_T=""
k=0
while :; do
    T_SNAP="$(now)"
    snapshot
    select_now
    if [ "$k" -eq 0 ] && [ ! -s "$SEL" ]; then
        die "no process found. Is the application running?"
    fi
    collect_footprints
    ISO="$(date +%Y-%m-%dT%H:%M:%S%z)"
    ELAPSED="$(calc "$T_SNAP - $START")"
    if [ -n "$PREV_T" ]; then
        DT="$(calc "$T_SNAP - $PREV_T")"
    else
        DT=0
    fi
    awk -v prevf="$PREV" -v fpf="$FPF" -v csv="$OUT" -v tsv="$DATA" -v prevnew="$PREVNEW" \
        -v sample="$((k + 1))" -v ts="$ISO" -v el="$ELAPSED" -v dt="$DT" '
        function clean(s) { gsub(/[,"\t]/, "_", s); return s }
        FILENAME == prevf { prev[$1] = $2; next }
        FILENAME == fpf { fp[$1] = $2; next }
        {
            pid = $1; ppid = $2; rss = $3; cpu = $4
            name = $0
            for (i = 0; i < 4; i++) sub(/^[^ ]+ +/, "", name)
            sub(/.*\//, "", name)
            name = clean(name)
            pct = ""
            if (dt + 0 > 0) {
                d = (pid in prev) ? cpu - prev[pid] : cpu
                if (d < 0) d = 0
                pct = sprintf("%.1f", 100 * d / dt)
                tcpu += d / dt * 100
            }
            f = (pid in fp) ? fp[pid] : ""
            if (f == "") miss++; else tfp += f
            trss += rss; tct += cpu; n++
            printf "%d,%s,%.1f,%s,%s,%s,%s,%s,%s,%s\n", sample, ts, el, pid, ppid, name, rss, f, cpu, pct >> csv
            printf "P\t%d\t%s\t%s\t%s\t%s\t%s\n", sample, pid, name, rss, f, pct >> tsv
            print pid, cpu >> prevnew
        }
        END {
            tp = (dt + 0 > 0) ? sprintf("%.1f", tcpu) : ""
            tf = (n > 0 && miss == n) ? "" : sprintf("%d", tfp)
            printf "%d,%s,%.1f,TOTAL,,TOTAL of %d processes,%d,%s,%.2f,%s\n", sample, ts, el, n, trss, tf, tct, tp >> csv
            printf "T\t%d\t%d\t%d\t%d\t%s\t%d\n", sample, n, trss, tfp, tp, miss >> tsv
        }' "$PREV" "$FPF" "$SEL"
    # With no process the awk script writes no prevnew file.
    if [ -f "$PREVNEW" ]; then
        mv "$PREVNEW" "$PREV"
    else
        : >"$PREV"
    fi
    PREV_T="$T_SNAP"
    k=$((k + 1))
    NEXT="$(calc "$k * $INTERVAL")"
    if awk -v n="$NEXT" -v d="$DURATION" 'BEGIN { exit !(n > d) }'; then
        break
    fi
    WAIT="$(calc "$START + $NEXT - $(now)")"
    if awk -v w="$WAIT" 'BEGIN { exit !(w > 0) }'; then
        sleep "$WAIT"
    fi
done

echo
echo "Measurement done. CSV file: $OUT"
summary
