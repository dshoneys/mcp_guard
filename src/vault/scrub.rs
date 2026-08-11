//! Redact secret material from process output before it can reach an LLM host.

pub fn scrub_secret(text: &str, secret: &str) -> String {
    if secret.is_empty() {
        return text.to_string();
    }
    text.replace(secret, "***REDACTED***")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_all_occurrences() {
        let out = scrub_secret("token=abc abc done", "abc");
        assert_eq!(out, "token=***REDACTED*** ***REDACTED*** done");
        assert!(!out.contains("abc"));
    }
}
