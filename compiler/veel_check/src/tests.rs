//! Unit tests of the checker's codes and value readers.

use super::*;

#[test]
fn no_code_is_also_the_syntaxs() {
    for code in codes::ALL {
        assert!(!veel_syntax::codes::ALL.contains(&code), "{code}");
        assert!(code.starts_with("veel-"));
    }
    let mut all = codes::ALL.to_vec();
    all.sort();
    all.dedup();
    assert_eq!(all.len(), codes::ALL.len(), "a code listed twice");
}

#[test]
fn every_code_veel_time_gives_is_the_checkers() {
    for code in ["veel-time-inexact", "veel-unit", "veel-range", "veel-type"] {
        assert!(codes::ALL.contains(&code));
        assert_eq!(codes::of_time(code), code);
    }
}

#[test]
fn numbers_and_their_units() {
    assert_eq!(values::number("-96"), Some((-96.0, "")));
    assert_eq!(values::number("85%"), Some((85.0, "%")));
    assert_eq!(values::number("2.5deg"), Some((2.5, "deg")));
    assert_eq!(values::number("1e-3"), Some((0.001, "")));
    assert_eq!(values::number(".5"), Some((0.5, "")));
    assert_eq!(values::number("deg"), None);
    assert_eq!(values::number("-"), None);
}

#[test]
fn kebab_spellings() {
    assert_eq!(read::kebab("Bad_Name").as_deref(), Some("bad-name"));
    assert_eq!(read::kebab("__"), None);
}
