//! Input checks shared by more than one domain.

const EMAIL_MAX: usize = 254;

/// An account email as it is stored and looked up: trimmed and lower case.
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Checks the shape of an address, not whether it exists: one `@` with
/// something before it, a dotted domain after it, and no spaces or control
/// characters.
pub fn is_valid_email(email: &str) -> bool {
    if email.len() > EMAIL_MAX
        || email
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return false;
    }
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && domain.split('.').all(|label| !label.is_empty())
}

/// Whether an optional text field holds more than `max` characters.
pub fn is_too_long(value: Option<&str>, max: usize) -> bool {
    value.is_some_and(|text| text.chars().count() > max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_shape_is_checked() {
        for good in ["ada@example.com", "a.b+c@mail.example.co.uk"] {
            assert!(is_valid_email(good), "{good}");
        }
        let long = format!("{}@example.com", "a".repeat(250));
        for bad in [
            "",
            "ada",
            "@example.com",
            "ada@",
            "ada@example",
            "ada@.com",
            "ada@example.",
            "ada@example..com",
            "ada@@example.com",
            "ada@b@example.com",
            "ada lovelace@example.com",
            " ada@example.com",
            "ada@example.com\n",
            "ada\u{0}@example.com",
            long.as_str(),
        ] {
            assert!(!is_valid_email(bad), "{bad:?}");
        }
    }

    #[test]
    fn account_emails_are_trimmed_and_lowered() {
        assert_eq!(normalize_email("  Ada@Example.COM "), "ada@example.com");
    }

    #[test]
    fn length_is_counted_in_characters() {
        assert!(!is_too_long(None, 1));
        assert!(!is_too_long(Some("გი"), 2));
        assert!(is_too_long(Some("გიო"), 2));
    }
}
