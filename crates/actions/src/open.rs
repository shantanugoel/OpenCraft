//! The rules of the action "Open an application": what a target is, and how
//! the arguments split. The functions are pure, so a test needs no system.

/// True if `target` names an application: a name with no `/` (for example
/// `Safari`), or a path of a bundle that ends with `.app`. Each other target
/// is a file or a folder.
pub(crate) fn is_application(target: &str) -> bool {
    let target = target.trim().trim_end_matches('/');
    !target.contains('/') || target.ends_with(".app")
}

/// Split `text` into arguments. A space, a tab, or a line end separates two
/// arguments. A pair of `"` or of `'` joins the characters in it into one
/// argument, and a `\` before a `"`, a `'`, a `\`, or a space gives that
/// character. `None` means that a quote has no end.
pub(crate) fn split_arguments(text: &str) -> Option<Vec<String>> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    // An empty pair of quotes is an argument too.
    let mut started = false;
    let mut quote: Option<char> = None;
    let mut characters = text.chars();
    while let Some(c) = characters.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (None, '"' | '\'') => {
                quote = Some(c);
                started = true;
            }
            (_, '\\') => match characters.clone().next() {
                Some(next @ ('"' | '\'' | '\\' | ' ')) => {
                    current.push(next);
                    started = true;
                    characters.next();
                }
                _ => current.push('\\'),
            },
            (None, c) if c.is_whitespace() => {
                if started || !current.is_empty() {
                    arguments.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            (_, c) => current.push(c),
        }
    }
    if quote.is_some() {
        return None;
    }
    if started || !current.is_empty() {
        arguments.push(current);
    }
    Some(arguments)
}
