//! Where one operation's caller input comes from.
//!
//! Three sources, because one of them stops working at scale. `--input-json` inlines the object and
//! is what a person types by hand; a real payload exceeds the operating system's argument-list
//! limit, at which point the failure happens in the shell rather than in this program, with a
//! message about `execve` rather than about the request. `--input-file` and stdin are the ways past
//! that, and stdin is the one a pipeline reaches for.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("caller input is required: pass --input-json, --input-file, or --input -")]
    Missing,
    #[error("the input file could not be read: {0}")]
    Io(#[from] std::io::Error),
    #[error("caller input is not a JSON object: {0}")]
    Json(#[from] serde_json::Error),
}

/// Resolve caller input from whichever source was named.
///
/// Clap's argument group guarantees at most one is present. It cannot express *at least* one while
/// `--input-json` stays optional for backward compatibility, so the "none of them" case is checked
/// here and refused by naming all three.
///
/// # Errors
///
/// [`InputError::Missing`] when no source was given, or the underlying read/parse failure.
pub fn read(
    inline: Option<String>,
    file: Option<PathBuf>,
    stdin_marker: Option<String>,
) -> Result<serde_json::Value, InputError> {
    let text = match (inline, file, stdin_marker) {
        (Some(inline), _, _) => inline,
        (_, Some(path), _) => std::fs::read_to_string(path)?,
        (_, _, Some(marker)) if marker == "-" => {
            let mut buffer = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin().lock(), &mut buffer)?;
            buffer
        }
        // `--input` exists to mean stdin and nothing else. Accepting a JSON document there would
        // give the same value two spellings, and the one that looks like a filename would silently
        // be parsed as JSON.
        (_, _, Some(_)) => return Err(InputError::Missing),
        (None, None, None) => return Err(InputError::Missing),
    };
    Ok(serde_json::from_str(&text)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_inline_object_is_parsed() {
        let value = read(Some(r#"{"query":"up"}"#.to_owned()), None, None).unwrap();
        assert_eq!(value, json!({"query": "up"}));
    }

    #[test]
    fn a_file_is_read_from_its_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("input.json");
        std::fs::write(&path, r#"{"project":"x"}"#).unwrap();
        let value = read(None, Some(path), None).unwrap();
        assert_eq!(value, json!({"project": "x"}));
    }

    #[test]
    fn no_source_names_all_three_rather_than_defaulting_to_empty() {
        // Defaulting to `{}` would send a request the operator did not write, and the vendor's
        // refusal would be about a missing field rather than about the missing input.
        let error = read(None, None, None).unwrap_err();
        assert!(matches!(error, InputError::Missing));
        assert!(error.to_string().contains("--input-file"));
    }

    #[test]
    fn input_accepts_only_the_stdin_marker() {
        let error = read(None, None, Some(r#"{"a":1}"#.to_owned())).unwrap_err();
        assert!(matches!(error, InputError::Missing));
    }
}
