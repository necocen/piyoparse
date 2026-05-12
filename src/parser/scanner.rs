/// Small consuming cursor for hand-written PiyoLog parsers.
///
/// `Scanner` keeps the unconsumed suffix of an input string and advances it
/// only through explicit `take_*`, `strip_prefix`, and `skip_*` calls. It is
/// intentionally tiny: parser functions compose these primitives instead of
/// carrying a general tokenizer or regex engine.
#[derive(Debug, Clone, Copy)]
pub(super) struct Scanner<'a> {
    rest: &'a str,
}

impl<'a> Scanner<'a> {
    /// Starts scanning at the beginning of `rest`.
    pub(super) fn new(rest: &'a str) -> Self {
        Self { rest }
    }

    /// Returns the unconsumed suffix.
    pub(super) fn rest(&self) -> &'a str {
        self.rest
    }

    /// Consumes `prefix` when it is present.
    ///
    /// Returns `false` without advancing when the next bytes do not match.
    pub(super) fn strip_prefix(&mut self, prefix: &str) -> bool {
        if let Some(rest) = self.rest.strip_prefix(prefix) {
            self.rest = rest;
            true
        } else {
            false
        }
    }

    /// Consumes PiyoLog spacing characters from the current position.
    pub(super) fn skip_spaces(&mut self) {
        self.rest = self.rest.trim_start_matches(is_piyolog_space);
    }

    /// Consumes a run of PiyoLog spacing characters and returns its char count.
    pub(super) fn take_space_run(&mut self) -> usize {
        let mut count = 0;
        let mut end = 0;

        for (index, character) in self.rest.char_indices() {
            if !is_piyolog_space(character) {
                break;
            }
            count += 1;
            end = index + character.len_utf8();
        }

        self.rest = &self.rest[end..];
        count
    }

    /// Consumes and returns the next character when `predicate` accepts it.
    ///
    /// Returns `None` without advancing when the next character is absent or
    /// rejected.
    pub(super) fn take_char_if(&mut self, predicate: impl FnOnce(char) -> bool) -> Option<char> {
        let mut characters = self.rest.char_indices();
        let (_, character) = characters.next()?;
        if !predicate(character) {
            return None;
        }

        let next = characters
            .next()
            .map(|(index, _)| index)
            .unwrap_or(self.rest.len());
        self.rest = &self.rest[next..];
        Some(character)
    }

    /// Consumes and returns a non-empty ASCII digit prefix.
    pub(super) fn take_ascii_digits(&mut self) -> Option<&'a str> {
        let end = self
            .rest
            .as_bytes()
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if end == 0 {
            return None;
        }

        let (digits, rest) = self.rest.split_at(end);
        self.rest = rest;
        Some(digits)
    }

    /// Consumes exactly `length` ASCII digits.
    ///
    /// Returns `None` without advancing unless all requested bytes are digits.
    pub(super) fn take_ascii_digits_exact(&mut self, length: usize) -> Option<&'a str> {
        if self.rest.len() < length
            || !self.rest.as_bytes()[..length]
                .iter()
                .all(|byte| byte.is_ascii_digit())
        {
            return None;
        }

        let (digits, rest) = self.rest.split_at(length);
        self.rest = rest;
        Some(digits)
    }

    /// Consumes an ASCII digit prefix and parses it as `u32`.
    pub(super) fn take_u32(&mut self) -> Option<u32> {
        self.take_ascii_digits()?.parse().ok()
    }
}

/// Returns whether `character` is spacing produced by PiyoLog exports.
pub(super) fn is_piyolog_space(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\u{3000}')
}

/// Trims PiyoLog spacing characters from the end of `value`.
pub(super) fn trim_end_piyolog_spaces(value: &str) -> &str {
    value.trim_end_matches(is_piyolog_space)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_prefix_advances_only_on_match() {
        let mut scanner = Scanner::new("母乳合計 左 5分");

        assert!(scanner.strip_prefix("母乳合計"));
        assert_eq!(scanner.rest(), " 左 5分");
        assert!(!scanner.strip_prefix("右"));
        assert_eq!(scanner.rest(), " 左 5分");
    }

    #[test]
    fn skips_and_counts_piyolog_spaces() {
        let mut scanner = Scanner::new(" \t　12分");

        assert_eq!(scanner.take_space_run(), 3);
        assert_eq!(scanner.rest(), "12分");

        let mut scanner = Scanner::new("　\t 12分");
        scanner.skip_spaces();
        assert_eq!(scanner.rest(), "12分");
    }

    #[test]
    fn take_char_if_advances_only_when_predicate_matches() {
        let mut scanner = Scanner::new("左 5分");

        assert_eq!(scanner.take_char_if(|character| character == '右'), None);
        assert_eq!(scanner.rest(), "左 5分");
        assert_eq!(
            scanner.take_char_if(|character| character == '左'),
            Some('左')
        );
        assert_eq!(scanner.rest(), " 5分");
    }

    #[test]
    fn takes_ascii_digits() {
        let mut scanner = Scanner::new("012分");

        assert_eq!(scanner.take_ascii_digits_exact(2), Some("01"));
        assert_eq!(scanner.rest(), "2分");
        assert_eq!(scanner.take_u32(), Some(2));
        assert_eq!(scanner.rest(), "分");
    }

    #[test]
    fn exact_digits_do_not_advance_on_short_or_non_digit_input() {
        let mut scanner = Scanner::new("7分");

        assert_eq!(scanner.take_ascii_digits_exact(2), None);
        assert_eq!(scanner.rest(), "7分");

        let mut scanner = Scanner::new("7a分");
        assert_eq!(scanner.take_ascii_digits_exact(2), None);
        assert_eq!(scanner.rest(), "7a分");
    }

    #[test]
    fn trims_piyolog_spaces_from_end() {
        assert_eq!(trim_end_piyolog_spaces("180ml \t　"), "180ml");
        assert_eq!(trim_end_piyolog_spaces("180ml\n"), "180ml\n");
    }
}
