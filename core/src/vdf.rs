//! The little of Valve's KeyValues format that `libraryfolders.vdf` needs.

use std::path::PathBuf;

/// Every `"path"` value in a `libraryfolders.vdf`, with `\\` unescaped.
pub fn library_paths(vdf_text: &str) -> Vec<PathBuf> {
    let tokens = tokens(vdf_text);
    tokens
        .windows(2)
        .filter(|w| w[0].eq_ignore_ascii_case("path"))
        .map(|w| PathBuf::from(&w[1]))
        .collect()
}

/// Quoted strings, unescaped; braces are skipped, which is all these files need.
pub(crate) fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut s = String::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => s.extend(chars.next()),
                _ => s.push(c),
            }
        }
        out.push(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_library_path() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t\t\"apps\" { \"228980\" \"0\" }\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t}\n}";
        assert_eq!(
            library_paths(vdf),
            vec![PathBuf::from(r"C:\Program Files (x86)\Steam"), PathBuf::from(r"D:\SteamLibrary")]
        );
    }
}
