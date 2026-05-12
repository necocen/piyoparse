#[derive(Debug, Clone, Copy)]
pub(super) struct Scanner<'a> {
    rest: &'a str,
}

impl<'a> Scanner<'a> {
    pub(super) fn new(rest: &'a str) -> Self {
        Self { rest }
    }

    pub(super) fn rest(&self) -> &'a str {
        self.rest
    }

    pub(super) fn strip_prefix(&mut self, prefix: &str) -> bool {
        if let Some(rest) = self.rest.strip_prefix(prefix) {
            self.rest = rest;
            true
        } else {
            false
        }
    }

    pub(super) fn skip_spaces(&mut self) {
        self.rest = self.rest.trim_start_matches(is_piyolog_space);
    }

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

    pub(super) fn take_i32(&mut self) -> Option<i32> {
        self.take_ascii_digits()?.parse().ok()
    }

    pub(super) fn take_u32(&mut self) -> Option<u32> {
        self.take_ascii_digits()?.parse().ok()
    }
}

pub(super) fn is_piyolog_space(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\u{3000}')
}

pub(super) fn trim_end_piyolog_spaces(value: &str) -> &str {
    value.trim_end_matches(is_piyolog_space)
}
