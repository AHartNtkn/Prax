//! The GateSpec `.rs`-scanner half (owed:S9), retargeted from the frozen
//! `src/Prax/Worlds/*.hs` to `rust/prax-worlds/src/*.rs`.
//!
//! The frozen `Prax.GateSpec` (`test/Prax/GateSpec.hs`) reads the world sources,
//! extracts quoted string-literal content by a naive unescaped-quote scan, and
//! flags any `Prax[A-Z][A-Za-z0-9]*`-shaped token inside a literal — ignoring
//! unquoted text (imports, comments) even if Prax-shaped. It is deliberately
//! naive and acceptable-conservative: false positives are fine (the gate's own
//! charter), ABSENCE of the scanner is not.
//!
//! **Scope: `prax-worlds/src` only, by PRINCIPLE not incumbency ([P9]b).** The
//! frozen scanned `Worlds/` only; the Rust reason is that `prax-vocab` is the
//! BUILDER layer — its combinator modules' job IS authoring Prax vocabulary, so
//! scanning them would flag the builders doing their work. The worlds are the
//! authored DATA, where a Prax-namespaced variable in a literal is the mistake
//! the gate exists to catch.
//!
//! **[P7]: Rust raw strings.** The frozen scanner is tuned to Haskell literals;
//! Rust adds raw strings (`r"…"`, `r#"…"#`) with different tokenization. The
//! discriminator subgroup below includes a raw-string case containing a
//! Prax-shaped token AND an inner unescaped quote, proving the scanner tokenizes
//! Rust string syntax correctly — else its Rust-fidelity would be unproven.
//!
//! The shared-guard half (`Prax.Types.authoredVarClash` through `draw`) already
//! landed at S4; this file is ONLY the scanner half.

/// Every quoted string literal's content in a Rust source file, found by a naive
/// scan. Handles BOTH ordinary `"…"` (with `\` escapes) and raw strings
/// (`r"…"`, `r#"…"#`, `r##"…"##`, … — no escapes, `#`-balanced close), so a
/// raw literal's inner unescaped `"` does not falsely terminate it ([P7]).
pub fn string_literals(src: &str) -> Vec<String> {
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        // Raw string: `r` at a token boundary, then zero+ `#`, then `"`.
        if chars[i] == 'r'
            && (i == 0 || !is_ident_continue(chars[i - 1]))
            && let Some((content, next)) = capture_raw(&chars, i)
        {
            out.push(content);
            i = next;
            continue;
        }
        if chars[i] == '"' {
            let (content, next) = capture_ordinary(&chars, i + 1);
            out.push(content);
            i = next;
            continue;
        }
        i += 1;
    }
    out
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Capture a raw string starting at `chars[i] == 'r'`. Returns the content and
/// the index just past the closing `"#…`, or `None` if `i` is not a raw-string
/// opener.
fn capture_raw(chars: &[char], i: usize) -> Option<(String, usize)> {
    let n = chars.len();
    let mut j = i + 1;
    let mut hashes = 0;
    while j < n && chars[j] == '#' {
        hashes += 1;
        j += 1;
    }
    if j >= n || chars[j] != '"' {
        return None;
    }
    let start = j + 1;
    // The close is `"` followed by exactly `hashes` `#`.
    let mut k = start;
    while k < n {
        if chars[k] == '"' {
            let mut h = 0;
            while h < hashes && k + 1 + h < n && chars[k + 1 + h] == '#' {
                h += 1;
            }
            if h == hashes {
                let content: String = chars[start..k].iter().collect();
                return Some((content, k + 1 + hashes));
            }
        }
        k += 1;
    }
    // Unterminated raw string: capture to end (loud-conservative, never silent).
    Some((chars[start..].iter().collect(), n))
}

/// Capture an ordinary string starting just past the opening `"` (at `start`).
/// A `\` escapes the next char (pushed literally, the frozen's naive rule);
/// the first unescaped `"` closes.
fn capture_ordinary(chars: &[char], start: usize) -> (String, usize) {
    let n = chars.len();
    let mut content = String::new();
    let mut j = start;
    while j < n {
        match chars[j] {
            '\\' if j + 1 < n => {
                content.push(chars[j + 1]);
                j += 2;
            }
            '"' => return (content, j + 1),
            c => {
                content.push(c);
                j += 1;
            }
        }
    }
    (content, n) // unterminated: capture to end
}

/// Every `Prax[A-Z][A-Za-z0-9]*`-shaped token in a string — a substring scan,
/// not a path-segment parse (false positives acceptable-conservative).
pub fn prax_tokens(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if i + 4 < n
            && chars[i] == 'P'
            && chars[i + 1] == 'r'
            && chars[i + 2] == 'a'
            && chars[i + 3] == 'x'
            && chars[i + 4].is_uppercase()
        {
            let mut tok = String::from("Prax");
            tok.push(chars[i + 4]);
            let mut j = i + 5;
            while j < n && chars[j].is_alphanumeric() {
                tok.push(chars[j]);
                j += 1;
            }
            out.push(tok);
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Every Prax-namespaced token found inside any quoted string literal of a
/// source file's content — the world-source gate's actual check.
pub fn world_source_offenders(src: &str) -> Vec<String> {
    string_literals(src)
        .iter()
        .flat_map(|lit| prax_tokens(lit))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // H: GateSpec.hs "the scanner (mutation evidence: it must actually discriminate)"
    //
    // The frozen `Prax.Gate` scanner subgroup. Each case runs the scanner over a
    // synthetic in-test Rust source string, so the scanner is shown to actually
    // discriminate — or it is not a scanner.

    // H: GateSpec.hs "catches a Prax-namespaced token inside a quoted literal"
    #[test]
    fn catches_a_prax_namespaced_token_inside_a_quoted_literal() {
        assert_eq!(
            world_source_offenders(r#"let p = matches("foo.PraxD.bar");"#),
            ["PraxD"]
        );
    }

    // H: GateSpec.hs "catches more than one offender, in order"
    #[test]
    fn catches_more_than_one_offender_in_order() {
        assert_eq!(
            world_source_offenders(r#"vec![matches("a.PraxW.b"), matches("c.PraxF!d")]"#),
            ["PraxW", "PraxF"]
        );
    }

    // H: GateSpec.hs "ignores ordinary quoted literals with no Prax-shaped token"
    #[test]
    fn ignores_ordinary_quoted_literals_with_no_prax_shaped_token() {
        assert_eq!(
            world_source_offenders(
                r#"Action::new("[Actor]: greet [Other]").when([matches("at.Actor!P")])"#
            ),
            Vec::<String>::new()
        );
    }

    // H: GateSpec.hs "ignores unquoted text (imports, comments) even if Prax-shaped"
    #[test]
    fn ignores_unquoted_text_even_if_prax_shaped() {
        // `PraxTypes` is Prax-shaped but sits in unquoted code + a comment; with
        // no string literal to scan, the scanner correctly finds nothing.
        assert_eq!(
            world_source_offenders("use prax_core::schedule::sight_rule; // see PraxTypes"),
            Vec::<String>::new()
        );
    }

    /// [P7] NATIVE PIN — no frozen label. The frozen scanner is a Haskell-literal
    /// scan; this proves the Rust retarget tokenizes RAW strings correctly: a
    /// raw literal's content IS scanned and its inner UNESCAPED `"` does not
    /// terminate it (a naive `"`-only scan would mis-tokenize and miss the
    /// token). REDDENS UNDER: dropping the raw-string arm of `string_literals`
    /// (the token would then be missed, or the inner quote would split the scan).
    #[test]
    fn p7_a_raw_string_literals_content_is_scanned_with_its_inner_quote() {
        // The raw literal `r#"a"PraxW"#` carries an inner UNESCAPED `"` BEFORE the
        // Prax token. This is the discriminating position: under a naive
        // ordinary-only scan the first `"` opens and the inner `"` CLOSES, leaving
        // `PraxW` in an (apparently) unquoted region that the scanner would MISS.
        // Only correct raw-string tokenization — content is `a"PraxW`, the inner
        // quote is literal — surfaces the token. So this reddens if the raw-string
        // arm of `string_literals` is dropped.
        assert_eq!(
            world_source_offenders(r##"matches(r#"a"PraxW"#)"##),
            ["PraxW"]
        );
        // A bare raw string with no hashes, too.
        assert_eq!(world_source_offenders(r#"m(r"a.PraxD.b")"#), ["PraxD"]);
    }

    // H: GateSpec.hs "no world source file authors a Prax-namespaced variable in a quoted literal"
    #[test]
    fn no_world_source_authors_a_prax_namespaced_variable_in_a_literal() {
        let worlds_dir = crate::source_sweep::rust_root().join("prax-worlds/src");
        let files = crate::source_sweep::every_rust_source(&worlds_dir);
        // The frozen's own anti-vacuity guard: an empty scan passes trivially.
        assert!(
            !files.is_empty(),
            "at least one world source file must exist under {} (an empty scan \
             would be vacuous)",
            worlds_dir.display()
        );
        let mut violations = Vec::new();
        for f in &files {
            let body = std::fs::read_to_string(f).expect("readable world source");
            let offenders = world_source_offenders(&body);
            if !offenders.is_empty() {
                violations.push(format!("{}: {offenders:?}", f.display()));
            }
        }
        assert!(
            violations.is_empty(),
            "Prax-namespaced variable(s) found in authored world source: {violations:?}"
        );
    }
}
