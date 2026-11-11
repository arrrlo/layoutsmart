use super::*;

#[test]
fn accepts_and_trims() {
    assert_eq!(validate("  home "), Ok("home".to_string()));
    assert_eq!(validate("no monitors"), Ok("no monitors".to_string()));
    assert_eq!(validate("a.b-c"), Ok("a.b-c".to_string()));
}

#[test]
fn length_limit_counts_characters_not_bytes() {
    assert_eq!(validate(&"a".repeat(MAX_LEN)), Ok("a".repeat(MAX_LEN)));
    assert_eq!(validate(&"é".repeat(MAX_LEN)), Ok("é".repeat(MAX_LEN)));
    assert_eq!(validate(&"a".repeat(MAX_LEN + 1)), Err(NameError::TooLong));
}

#[test]
fn rejects_bad_names() {
    assert_eq!(validate(""), Err(NameError::Empty));
    assert_eq!(validate("   "), Err(NameError::Empty));
    assert_eq!(validate("a/b"), Err(NameError::InvalidChar('/')));
    assert_eq!(validate("a:b"), Err(NameError::InvalidChar(':')));
    assert_eq!(validate("a\tb"), Err(NameError::InvalidChar('\t')));
    assert_eq!(validate(".hidden"), Err(NameError::InvalidStart('.')));
    assert_eq!(validate("-v"), Err(NameError::InvalidStart('-')));
    assert_eq!(validate("save"), Err(NameError::Reserved("save".into())));
    assert_eq!(
        validate(" profiles "),
        Err(NameError::Reserved("profiles".into()))
    );
}

#[test]
fn reserved_words_cover_all_commands() {
    for w in [
        "save", "restore", "list", "displays", "profiles", "rename", "delete", "help",
    ] {
        assert!(is_reserved(w), "{w}");
    }
    assert!(!is_reserved("home"));
    assert!(!is_reserved("Save"));
}

#[test]
fn messages() {
    assert_eq!(NameError::Empty.to_string(), "Profile name can't be empty.");
    assert_eq!(
        NameError::TooLong.to_string(),
        "Profile name can be at most 64 characters."
    );
    assert_eq!(
        NameError::InvalidChar('/').to_string(),
        "Profile name can't contain '/'."
    );
    assert_eq!(
        NameError::InvalidChar('\t').to_string(),
        "Profile name can't contain '\\t'."
    );
    assert_eq!(
        NameError::InvalidStart('.').to_string(),
        "Profile name can't start with '.'."
    );
    assert_eq!(
        NameError::Reserved("save".into()).to_string(),
        "'save' is a command and can't be a profile name."
    );
}

#[test]
fn error_is_cloneable_and_printable() {
    assert_eq!(NameError::Empty.clone(), NameError::Empty);
    assert_eq!(format!("{:?}", NameError::Empty), "Empty");
}
