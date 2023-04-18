use core::iter::Peekable;
use core::ops::Range;
use core::str::CharIndices;

#[derive(Debug, Clone)]
pub struct CharIterator<'a> {
    pub source: &'a str,
    chars: Peekable<CharIndices<'a>>,
    last: Option<(usize, char)>,
    marked_start: usize,
}

impl<'a> CharIterator<'a> {
    #[inline]
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            last: None,
            marked_start: 0,
        }
    }

    #[inline]
    pub const fn last_offset(&self) -> usize {
        if let Some((offset, _)) = self.last {
            offset
        } else {
            0
        }
    }

    #[inline]
    pub const fn current_offset(&self) -> usize {
        if let Some((offset, ch)) = self.last {
            offset + ch.len_utf8()
        } else {
            0
        }
    }

    #[inline]
    pub fn mark_start(&mut self) {
        self.marked_start = self.current_offset();
    }

    #[inline]
    pub const fn marked_range(&self) -> Range<usize> {
        self.marked_start..self.current_offset()
    }

    pub fn marked_str(&self) -> &'a str {
        &self.source[self.marked_range()]
    }

    #[inline]
    pub const fn last_char_range(&self) -> Range<usize> {
        if let Some((offset, ch)) = self.last {
            offset..(offset + ch.len_utf8())
        } else {
            0..0
        }
    }

    #[inline]
    pub fn next_char_and_index(&mut self) -> Option<(usize, char)> {
        let current = self.chars.next()?;
        self.last = Some(current);
        Some(current)
    }

    #[inline]
    pub fn peek(&mut self) -> Option<char> {
        self.peek_full().map(|(_, ch)| ch)
    }

    #[inline]
    pub fn peek_full(&mut self) -> Option<(usize, char)> {
        self.chars.peek().copied()
    }
}

impl<'a> Iterator for CharIterator<'a> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.next_char_and_index().map(|(_, ch)| ch)
    }
}
