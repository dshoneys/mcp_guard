use mcp_guard::git_scan::detect_in_bytes;

fn fake_sig(n: usize) -> String {
    // deterministic b64-ish padding
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        .chars()
        .cycle()
        .take(n)
        .collect()
}

#[test]
fn detects_anthropic_thinking_signature() {
    let sig = fake_sig(120);
    let body = format!(
        r#"{{"type":"thinking","thinking":"secret","signature":"{sig}"}}"#,
    );
    let hits = detect_in_bytes(body.as_bytes());
    assert!(
        hits.iter()
            .any(|h| h.provider_hint == "anthropic_thinking_signature" && h.token_len >= 80),
        "hits={hits:?}"
    );
}

#[test]
fn detects_openai_encrypted_content() {
    let sig = fake_sig(96);
    let body = format!(r#"{{"reasoning":{{"encrypted_content":"{sig}"}}}}"#);
    let hits = detect_in_bytes(body.as_bytes());
    assert!(hits.iter().any(|h| h.provider_hint == "openai_encrypted_content"));
}

#[test]
fn detects_gemini_thought_signature() {
    let sig = fake_sig(48);
    let body = format!(r#"{{"thoughtSignature":"{sig}"}}"#);
    let hits = detect_in_bytes(body.as_bytes());
    assert!(hits.iter().any(|h| h.provider_hint == "gemini_thought_signature"));
}

#[test]
fn ignores_short_signature_noise() {
    let body = r#"{"signature":"short"}"#;
    let hits = detect_in_bytes(body.as_bytes());
    assert!(hits.is_empty());
}

#[test]
fn case_fixture_is_flagged() {
    let fixture = include_str!("../cases/arxiv-2608-09867/fixtures/sample_thinking_leak.json");
    let hits = detect_in_bytes(fixture.as_bytes());
    assert!(
        !hits.is_empty(),
        "case fixture must contain a detectable signature"
    );
}
