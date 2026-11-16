//! The skill for an agent (decision 0040, section 5).
//!
//! The text is the file `skills/opencraft/SKILL.md` of the repository. The
//! build puts it into the executable, thus it always agrees with the commands
//! of this version. `opencraft ctl skill` writes it to the standard output.
//! `opencraft ctl skill --install <directory>` writes it to
//! `<directory>/opencraft/SKILL.md`: the directory is the directory of the
//! skills of an agent, for example the directory `skills` of its settings.

use super::status;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

/// The text of the skill, with its own version.
pub const TEXT: &str = include_str!("../../../../skills/opencraft/SKILL.md");

/// The name of the directory of the skill. It is the `name` in the text.
const SKILL_DIRECTORY: &str = "opencraft";
const FILE: &str = "SKILL.md";

/// The help of `opencraft ctl skill`.
pub fn help() -> String {
    format!(
        "opencraft ctl skill\n\n\
         Write the skill for an agent to the standard output. The skill has the\n\
         commands of this version, the terms, and the limits.\n\n\
         Arguments:\n  \
         --install DIRECTORY  Write the skill to DIRECTORY/{SKILL_DIRECTORY}/{FILE} in place of the\n  \
         standard output. DIRECTORY is the directory of the skills of the agent.\n\n\
         Example:\n  opencraft ctl skill --install ~/.claude/skills\n"
    )
}

/// Run `opencraft ctl skill` with the arguments after `skill`. OpenCraft need
/// not run.
pub fn run(arguments: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match arguments {
        [] => {
            let _ = out.write_all(TEXT.as_bytes());
            status::DONE
        }
        [flag] if flag == "--help" || flag == "-h" => {
            let _ = out.write_all(help().as_bytes());
            status::DONE
        }
        [flag, directory] if flag == "--install" => {
            match super::parse::absolute(directory).and_then(|d| install(Path::new(&d))) {
                Ok(file) => {
                    let _ = writeln!(out, "{}", file.display());
                    status::DONE
                }
                Err(problem) => {
                    let _ = writeln!(err, "{problem}");
                    status::REFUSED
                }
            }
        }
        _ => {
            let _ = writeln!(
                err,
                "The arguments are not correct.\nSee `opencraft ctl skill --help`."
            );
            status::ARGUMENTS
        }
    }
}

/// Write the skill to `<directory>/opencraft/SKILL.md`. The result is the
/// file. An earlier file is replaced.
pub fn install(directory: &Path) -> Result<PathBuf, String> {
    let folder = directory.join(SKILL_DIRECTORY);
    fs::create_dir_all(&folder)
        .map_err(|e| format!("OpenCraft cannot make {}: {e}.", folder.display()))?;
    let file = folder.join(FILE);
    fs::write(&file, TEXT)
        .map_err(|e| format!("OpenCraft cannot write {}: {e}.", file.display()))?;
    Ok(file)
}
