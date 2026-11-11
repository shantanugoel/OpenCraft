//! Recorded input reports: the text format that `opencraft-probe raw --save`
//! writes, and a parser for it.
//!
//! The format is in `crates/core/tests/fixtures/input/README.md`. In short:
//!
//! - A line that starts with `#` is a comment.
//! - The first comment line is [`FORMAT_LINE`]. The next lines have the form
//!   `# note: <text>`, `# device: <name>`, and `# firmware: <version>`.
//! - Each other line is one report: the time in milliseconds from the start of
//!   the recording, one space, and all bytes of the report as hexadecimal with
//!   no separators.
//!
//! The parser has no dependency on a device. Tests can use it to read the
//! fixtures.

/// The first line of each recording file. It gives the format version.
pub const FORMAT_LINE: &str = "# opencraft-input-recording v1";

/// One recorded input report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Time in milliseconds from the start of the recording.
    pub ms: u64,
    /// All bytes of the report.
    pub bytes: Vec<u8>,
}

/// A parsed recording file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Recording {
    /// The gesture that the file holds.
    pub note: String,
    /// The device model name.
    pub device: String,
    /// The firmware version.
    pub firmware: String,
    pub reports: Vec<Report>,
}

/// Replace each line break and control character with a space, so that a text
/// stays on one line.
fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

/// The comment lines for the start of a recording file. Each line ends with a
/// line break.
pub fn header(note: &str, device: &str, firmware: &str) -> String {
    format!(
        "{FORMAT_LINE}\n# note: {}\n# device: {}\n# firmware: {}\n# columns: time_ms report_hex\n",
        one_line(note),
        one_line(device),
        one_line(firmware),
    )
}

/// One report as a line of a recording file, with no line break at the end.
pub fn format_report_line(ms: u64, bytes: &[u8]) -> String {
    let mut line = format!("{ms} ");
    line.reserve(bytes.len() * 2);
    for b in bytes {
        line.push_str(&format!("{b:02x}"));
    }
    line
}

/// Read one report line. Returns `None` if the line is not a report line.
/// Upper-case and lower-case hexadecimal are both valid.
pub fn parse_report_line(line: &str) -> Option<Report> {
    let (ms, hex) = line.trim().split_once(' ')?;
    let ms = ms.parse().ok()?;
    let hex = hex.trim();
    if hex.is_empty() || hex.len() % 2 != 0 || !hex.is_ascii() {
        return None;
    }
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect::<Option<Vec<u8>>>()?;
    Some(Report { ms, bytes })
}

/// Read a whole recording file.
pub fn parse(text: &str) -> Result<Recording, String> {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some(FORMAT_LINE) {
        return Err(format!("the first line must be: {FORMAT_LINE}"));
    }
    let mut rec = Recording::default();
    for (i, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            let comment = comment.trim_start();
            if let Some(v) = comment.strip_prefix("note:") {
                rec.note = v.trim().to_string();
            } else if let Some(v) = comment.strip_prefix("device:") {
                rec.device = v.trim().to_string();
            } else if let Some(v) = comment.strip_prefix("firmware:") {
                rec.firmware = v.trim().to_string();
            }
            continue;
        }
        let report =
            parse_report_line(line).ok_or_else(|| format!("line {}: not a report", i + 2))?;
        rec.reports.push(report);
    }
    Ok(rec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_a_report_line() {
        assert_eq!(format_report_line(0, &[0x41, 0x43, 0x4B]), "0 41434b");
        assert_eq!(format_report_line(1234, &[0x00, 0xFF, 0x0A]), "1234 00ff0a");
    }

    #[test]
    fn formats_all_bytes_not_only_32() {
        let bytes = vec![0xABu8; 512];
        let line = format_report_line(7, &bytes);
        assert_eq!(line.len(), "7 ".len() + 1024);
        assert_eq!(parse_report_line(&line).unwrap().bytes, bytes);
    }

    #[test]
    fn parses_a_report_line() {
        let r = parse_report_line("250 41434b0000").unwrap();
        assert_eq!(r.ms, 250);
        assert_eq!(r.bytes, vec![0x41, 0x43, 0x4B, 0, 0]);
        // Upper case is valid.
        assert_eq!(parse_report_line("1 FF0A").unwrap().bytes, vec![0xFF, 0x0A]);
    }

    #[test]
    fn rejects_bad_report_lines() {
        for bad in [
            "", "12", "12 ", "x 00", "12 0", "12 zz", "-1 00", "12 0é", "# 12 00",
        ] {
            assert_eq!(parse_report_line(bad), None, "line: {bad:?}");
        }
    }

    #[test]
    fn line_round_trips() {
        let bytes: Vec<u8> = (0..=255).collect();
        let r = parse_report_line(&format_report_line(99, &bytes)).unwrap();
        assert_eq!((r.ms, r.bytes), (99, bytes));
    }

    #[test]
    fn header_keeps_each_text_on_one_line() {
        let h = header("turn\nleft\r\tabout 5 clicks", "Model", "V1");
        assert_eq!(h.lines().count(), 5);
        assert!(h.starts_with(FORMAT_LINE));
        assert!(h.lines().all(|l| l.starts_with('#')));
        assert!(h.contains("# note: turn left  about 5 clicks\n"));
    }

    #[test]
    fn parses_a_whole_file() {
        let mut text = header("key 6 press and release", "Model X", "V1.2");
        text.push_str(&format_report_line(0, &[1, 2, 3]));
        text.push('\n');
        text.push_str(&format_report_line(80, &[4, 5]));
        text.push('\n');
        let rec = parse(&text).unwrap();
        assert_eq!(rec.note, "key 6 press and release");
        assert_eq!(rec.device, "Model X");
        assert_eq!(rec.firmware, "V1.2");
        assert_eq!(
            rec.reports,
            vec![
                Report {
                    ms: 0,
                    bytes: vec![1, 2, 3]
                },
                Report {
                    ms: 80,
                    bytes: vec![4, 5]
                }
            ]
        );
    }

    #[test]
    fn parses_a_file_with_no_reports() {
        let rec = parse(&header("skipped", "M", "F")).unwrap();
        assert!(rec.reports.is_empty());
    }

    #[test]
    fn rejects_a_wrong_version_and_a_bad_line() {
        assert!(parse("# opencraft-input-recording v2\n").is_err());
        assert!(parse("").is_err());
        let text = format!("{FORMAT_LINE}\n0 00\nnot a report\n");
        assert_eq!(parse(&text).unwrap_err(), "line 3: not a report");
    }
}
