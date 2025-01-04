//! src/domain/subscription_token.rs

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

#[derive(Debug)]
pub struct SubscriptionToken(String);

impl SubscriptionToken {
    pub fn parse(s: String) -> Result<Self, String> {
        let is_correct_length = s.len() == 25;
        let is_alphanumneric = s.chars().all(|c| c.is_alphanumeric());

        if is_correct_length && is_alphanumneric {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid subscription token.", s))
        }
    }

    pub fn generate() -> SubscriptionToken {
        let mut rng = thread_rng();
        let s = std::iter::repeat_with(|| rng.sample(Alphanumeric))
            .map(char::from)
            .take(25)
            .collect();
        Self::parse(s).unwrap()
    }
}

impl AsRef<str> for SubscriptionToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriptionToken;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_valid_token_is_valid() {
        let token = "abcdefghijklmnopqrstuv123".to_string();
        assert_ok!(SubscriptionToken::parse(token));
    }

    #[test]
    fn a_generated_token_is_valid() {
        let token = SubscriptionToken::generate();
        assert_ok!(SubscriptionToken::parse(token.0));
    }

    #[test]
    fn an_empty_token_is_rejected() {
        let token = "".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn a_short_token_is_rejected() {
        let token = "abc".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn a_long_token_is_rejected() {
        let token = "abcdefghijklmnopqrstuv1234".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn a_whitespace_token_is_rejected() {
        let token = "abcdefghijklmnopqrstuv12 ".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn a_non_alphanumeric_token_is_rejected() {
        let token = "abcdefghijklmnopqrstuv12.".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }
}
