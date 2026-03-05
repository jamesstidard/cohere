use std::collections::HashMap;

/// A source location in a document (JSON, TOML, etc.)
#[derive(Debug, Clone, Copy)]
pub struct SourceSpan {
    /// Byte offset of the start of the value
    pub start: usize,
    /// Byte offset of the end of the value (exclusive)
    pub end: usize,
    /// 1-indexed line number
    pub line: usize,
    /// 1-indexed column number
    pub column: usize,
}

/// Maps JSON paths (e.g. "users[0].name") to their source location
pub type SourceMap = HashMap<String, SourceSpan>;

/// Build a lookup table mapping line index to byte offset of the start of each line.
pub fn build_line_offsets(src: &str) -> Vec<usize> {
    let mut offsets = vec![0]; // line 1 starts at byte 0
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// Convert a byte offset to 1-indexed line and column numbers.
pub fn offset_to_line_col(offset: usize, line_offsets: &[usize]) -> (usize, usize) {
    let line_idx = match line_offsets.binary_search(&offset) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    let col = offset - line_offsets[line_idx];
    (line_idx + 1, col + 1) // 1-indexed
}

pub fn make_span(start: usize, end: usize, line_offsets: &[usize]) -> SourceSpan {
    let (line, column) = offset_to_line_col(start, line_offsets);
    SourceSpan {
        start,
        end,
        line,
        column,
    }
}
