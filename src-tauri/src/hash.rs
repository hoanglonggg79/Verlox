pub fn validate_hash(raw: &str) -> Option<String> {
    let h = raw.trim();
    if h.is_empty() {
        return None;
    }
    let h = h.strip_prefix("version-").unwrap_or(h);
    if h.len() == 16 && h.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("version-{h}"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_full_form() {
        assert_eq!(
            validate_hash("version-eb4648e9148d440c").as_deref(),
            Some("version-eb4648e9148d440c")
        );
    }

    #[test]
    fn adds_prefix_and_uppercase_ok() {
        assert_eq!(
            validate_hash("eb4648e9148d440c").as_deref(),
            Some("version-eb4648e9148d440c")
        );
        assert_eq!(
            validate_hash("EB4648E9148D440C").as_deref(),
            Some("version-EB4648E9148D440C")
        );
        assert!(validate_hash("  version-eb4648e9148d440c  ").is_some());
    }

    #[test]
    fn rejects_bad_hashes() {
        assert!(validate_hash("").is_none());
        assert!(validate_hash("version-abc123").is_none());
        assert!(validate_hash("1.2.3.4567").is_none());
        assert!(validate_hash("version-eb4648e9148d440").is_none());
        assert!(validate_hash("version-eb4648e9148d440z").is_none());
    }
}
