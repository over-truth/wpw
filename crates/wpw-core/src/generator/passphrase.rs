use rand::seq::SliceRandom;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct PassphraseOptions {
    pub word_count: usize,
    pub separator: char,
    pub capitalize: bool,
}

impl Default for PassphraseOptions {
    fn default() -> Self {
        Self {
            word_count: 5,
            separator: '-',
            capitalize: false,
        }
    }
}

/// Full EFF "large" Diceware wordlist (7776 words), embedded at build time from
/// `assets/wordlist/eff-large.txt`. The file format is `<dice-roll>\t<word>` per line;
/// we strip the dice column and keep the word.
///
/// Why this matters: the previous in-source list was ~1500 words (≈53 bit entropy for a
/// 5-word passphrase). The full EFF list gives log2(7776^5) ≈ 64.6 bits, matching
/// what the docs and Diceware spec promise.
const WORDLIST_RAW: &str = include_str!("../../../../assets/wordlist/eff-large.txt");

fn wordlist() -> &'static [&'static str] {
    static CACHE: OnceLock<Vec<&'static str>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let words: Vec<&'static str> = WORDLIST_RAW
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() { return None; }
                // Each line is "<dice>\t<word>"; take the last whitespace-separated token.
                trimmed.split_whitespace().last()
            })
            .collect();
        // Defence in depth: if the file ever drifts, fail loudly rather than silently
        // producing weaker passphrases.
        assert!(
            words.len() == 7776,
            "EFF wordlist must contain exactly 7776 words, found {}",
            words.len()
        );
        words
    })
}

/// Generate a random passphrase.
pub fn generate_passphrase(options: &PassphraseOptions) -> String {
    let mut rng = rand::thread_rng();
    let list = wordlist();

    let words: Vec<String> = (0..options.word_count)
        .map(|_| {
            // SliceRandom::choose uses an unbiased rejection sample, so each word is
            // equally likely.
            let word = *list.choose(&mut rng).expect("wordlist is non-empty");
            if options.capitalize {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                }
            } else {
                word.to_string()
            }
        })
        .collect();

    words.join(&options.separator.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wordlist_has_full_eff_set() {
        assert_eq!(wordlist().len(), 7776);
    }

    #[test]
    fn passphrase_has_requested_word_count() {
        let opts = PassphraseOptions { word_count: 6, separator: '-', capitalize: false };
        let p = generate_passphrase(&opts);
        assert_eq!(p.matches('-').count(), 5);
        assert!(p.split('-').all(|w| !w.is_empty()));
    }
}
