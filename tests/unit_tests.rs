use blerust::{Completer, History, LineBuffer, SyntaxHighlighter, TokenType};

#[test]
fn test_line_buffer_basic_ops() {
    let mut buf = LineBuffer::new();
    buf.insert_str("echo hello");
    assert_eq!(buf.as_str(), "echo hello");
    assert_eq!(buf.cursor(), 10);

    buf.move_word_left();
    assert_eq!(buf.cursor(), 5);

    buf.insert_str("world ");
    assert_eq!(buf.as_str(), "echo world hello");

    buf.move_cursor_end();
    assert!(buf.backspace());
    assert_eq!(buf.as_str(), "echo world hell");

    buf.undo();
    assert_eq!(buf.as_str(), "echo world hello");
}

#[test]
fn test_syntax_highlighting() {
    let hl = SyntaxHighlighter::new();
    let spans = hl.highlight("echo \"hello world\" | grep hello # test comment");
    assert!(!spans.is_empty());

    let has_builtin = spans.iter().any(|s| s.token_type == TokenType::Builtin && s.text == "echo");
    assert!(has_builtin);

    let has_string = spans.iter().any(|s| s.token_type == TokenType::StringLiteral);
    assert!(has_string);

    let has_op = spans.iter().any(|s| s.token_type == TokenType::Operator && s.text == "|");
    assert!(has_op);

    let has_comment = spans.iter().any(|s| s.token_type == TokenType::Comment);
    assert!(has_comment);
}

#[test]
fn test_completion_builtins_and_lcp() {
    let completer = Completer::new();
    let (start, matches) = completer.complete("ec", 2).expect("expected matches for 'ec'");
    assert_eq!(start, 0);
    assert!(matches.contains(&"echo".to_string()));

    let candidates = vec!["echo".to_string(), "echon".to_string()];
    let lcp = Completer::longest_common_prefix(&candidates);
    assert_eq!(lcp, "echo");
}

#[test]
fn test_history_suggestions() {
    let mut history = History::new();
    history.add("git status");
    history.add("git commit -m 'initial commit'");
    history.add("cargo build --release");

    assert_eq!(history.suggest_suffix("cargo "), Some("build --release".to_string()));
    assert_eq!(history.suggest_suffix("git c"), Some("ommit -m 'initial commit'".to_string()));
}
