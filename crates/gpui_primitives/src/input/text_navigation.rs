use gpui::SharedString;
use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Implemented as a trait to keep navigation logic separate from core state.
pub(crate) trait TextNavigation {
    fn value(&self) -> SharedString;

    fn line_count(&self) -> usize {
        let value = self.value();
        if value.is_empty() {
            1
        } else {
            value.chars().filter(|&c| c == '\n').count() + 1
        }
    }

    fn line_start_offset(&self, line: usize) -> usize {
        let value = self.value();
        let mut offset = 0;
        for (i, _) in value.split('\n').enumerate() {
            if i == line {
                return offset;
            }
            offset += value[offset..].find('\n').map(|p| p + 1).unwrap_or(0);
        }
        value.len()
    }

    fn line_end_offset(&self, line: usize) -> usize {
        let start = self.line_start_offset(line);
        let value = self.value();
        value[start..]
            .find('\n')
            .map(|p| start + p)
            .unwrap_or(value.len())
    }

    fn offset_to_line_col(&self, offset: usize) -> (usize, usize) {
        let value = self.value();
        let mut line = 0;
        let mut line_start = 0;

        for (i, c) in value.char_indices() {
            if i >= offset {
                break;
            }
            if c == '\n' {
                line += 1;
                line_start = i + 1;
            }
        }

        (line, offset.saturating_sub(line_start))
    }

    fn line_col_to_offset(&self, line: usize, col: usize) -> usize {
        let line_start = self.line_start_offset(line);
        let line_end = self.line_end_offset(line);
        let line_len = line_end - line_start;
        line_start + col.min(line_len)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.value()
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.value()
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.value().len())
    }

    fn word_start(&self, offset: usize) -> usize {
        let value = self.value();
        if value.is_empty() || offset == 0 {
            return 0;
        }

        let graphemes: Vec<(usize, &str)> = value
            .grapheme_indices(true)
            .take_while(|(i, _)| *i < offset)
            .collect();

        let Some(&(last_idx, last_grapheme)) = graphemes.last() else {
            return 0;
        };

        let last_char = last_grapheme.chars().next().unwrap_or(' ');
        if !is_word_char(last_char) {
            return last_idx;
        }

        for &(idx, grapheme) in graphemes.iter().rev() {
            let c = grapheme.chars().next().unwrap_or(' ');
            if !is_word_char(c) {
                return idx + grapheme.len();
            }
        }
        0
    }

    fn word_end(&self, offset: usize) -> usize {
        let value = self.value();
        if value.is_empty() || offset >= value.len() {
            return value.len();
        }

        let mut graphemes = value[offset..].grapheme_indices(true);
        let Some((_, first_grapheme)) = graphemes.next() else {
            return value.len();
        };

        let first_char = first_grapheme.chars().next().unwrap_or(' ');
        if !is_word_char(first_char) {
            return offset + first_grapheme.len();
        }

        for (i, grapheme) in value[offset..].grapheme_indices(true) {
            let c = grapheme.chars().next().unwrap_or(' ');
            if !is_word_char(c) {
                return offset + i;
            }
        }
        value.len()
    }
}
