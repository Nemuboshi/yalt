//! Incremental filter that removes inline reasoning blocks from provider text.
//!
//! The filter is fed arbitrary fragments: a tag may be split across any number
//! of calls, so text that could still become a tag is held back rather than
//! emitted early. Hidden reasoning is never released, and text that ends inside
//! an unfinished block is discarded instead of shown.

use std::fmt;

/// Opening tag that starts a hidden reasoning block.
const OPEN_TAG: &str = "<think>";
/// Closing tag that ends a hidden reasoning block.
const CLOSE_TAG: &str = "</think>";

/// Error raised when a response ends inside an unfinished reasoning block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnfinishedReasoning;

impl fmt::Display for UnfinishedReasoning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("response ended inside a reasoning block")
    }
}

impl std::error::Error for UnfinishedReasoning {}

/// Incremental reasoning filter.
#[derive(Debug, Default)]
pub struct ThinkFilter {
    /// Text received but not yet emitted or discarded.
    buffer: String,
    /// Whether the filter is currently inside a reasoning block.
    in_thinking: bool,
}

impl ThinkFilter {
    /// Create an empty filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a fragment and return the visible text it completes.
    ///
    /// Returns an empty string while the fragment only extends a tag or a
    /// hidden block.
    pub fn feed(&mut self, input: &str) -> String {
        self.buffer.push_str(input);
        let mut out = String::new();
        loop {
            if self.in_thinking {
                match self.buffer.find(CLOSE_TAG) {
                    Some(end) => {
                        self.buffer.drain(..end + CLOSE_TAG.len());
                        self.in_thinking = false;
                    }
                    None => {
                        // Keep only enough trailing text to complete a closing
                        // tag that may still be split; discard the rest.
                        self.trim_to_tag_suffix(CLOSE_TAG);
                        return out;
                    }
                }
                continue;
            }
            match self.buffer.find(OPEN_TAG) {
                Some(0) => {
                    self.buffer.drain(..OPEN_TAG.len());
                    self.in_thinking = true;
                }
                Some(start) => {
                    out.push_str(&self.buffer[..start]);
                    self.buffer.drain(..start);
                }
                None => {
                    // Emit everything that cannot still become an opening tag.
                    let keep = self.tag_prefix_len(OPEN_TAG);
                    let emit = self.buffer.len() - keep;
                    out.push_str(&self.buffer[..emit]);
                    self.buffer.drain(..emit);
                    return out;
                }
            }
        }
    }

    /// Flush the filter at the end of a response.
    ///
    /// Fails when the response ended inside an unfinished reasoning block, so
    /// hidden reasoning is never exposed.
    pub fn flush(&mut self) -> Result<String, UnfinishedReasoning> {
        if self.in_thinking {
            return Err(UnfinishedReasoning);
        }
        Ok(std::mem::take(&mut self.buffer))
    }

    /// Drop all buffered text except a suffix that may still complete `tag`.
    fn trim_to_tag_suffix(&mut self, tag: &str) {
        let keep = self.tag_prefix_len(tag);
        let drop = self.buffer.len() - keep;
        self.buffer.drain(..drop);
    }

    /// Length of the longest suffix of the buffer that is a prefix of `tag`.
    fn tag_prefix_len(&self, tag: &str) -> usize {
        let max = tag.len().min(self.buffer.len());
        for len in (1..=max).rev() {
            // Only split on character boundaries.
            if !self.buffer.is_char_boundary(self.buffer.len() - len) {
                continue;
            }
            let suffix = &self.buffer[self.buffer.len() - len..];
            if tag.starts_with(suffix) {
                return len;
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feed every fragment in order, returning the concatenated visible text.
    fn run(fragments: &[&str]) -> String {
        let mut filter = ThinkFilter::new();
        let mut out = String::new();
        for fragment in fragments {
            out.push_str(&filter.feed(fragment));
        }
        out.push_str(&filter.flush().expect("flush"));
        out
    }

    #[test]
    fn passes_through_plain_text() {
        assert_eq!(run(&["hello", " world"]), "hello world");
    }

    #[test]
    fn removes_a_complete_block() {
        assert_eq!(run(&["<think>secret</think>visible"]), "visible");
    }

    #[test]
    fn handles_a_split_opening_tag() {
        assert_eq!(run(&["<thi", "nk>secret</think>visible"]), "visible");
    }

    #[test]
    fn handles_a_split_closing_tag() {
        assert_eq!(run(&["<think>secret</thi", "nk>visible"]), "visible");
    }

    #[test]
    fn keeps_surrounding_visible_text() {
        assert_eq!(run(&["before<think>hidden</think>after"]), "beforeafter");
    }

    #[test]
    fn removes_multiple_blocks() {
        assert_eq!(run(&["a<think>1</think>b<think>2</think>c"]), "abc");
    }

    #[test]
    fn never_exposes_unfinished_reasoning() {
        let mut filter = ThinkFilter::new();
        let out = filter.feed("<think>secret that never closes");
        assert_eq!(out, "", "hidden reasoning leaked: {out:?}");
        assert!(filter.flush().is_err());
    }

    #[test]
    fn holds_back_a_partial_tag_suffix() {
        let mut filter = ThinkFilter::new();
        // "text<thi" could still become an opening tag, so "<thi" is withheld.
        assert_eq!(filter.feed("text<thi"), "text");
        // It resolves into a real block, so the block's content stays hidden and
        // only the text after it is emitted.
        assert_eq!(filter.feed("nk>secret</think>visible"), "visible");
        assert_eq!(filter.flush().unwrap(), "");
    }

    #[test]
    fn emits_text_that_only_looked_like_a_tag() {
        // A lone "<" that never becomes a tag is released on flush.
        let mut filter = ThinkFilter::new();
        assert_eq!(filter.feed("a<"), "a");
        assert_eq!(filter.flush().unwrap(), "<");
    }

    #[test]
    fn flush_returns_trailing_visible_text() {
        let mut filter = ThinkFilter::new();
        assert_eq!(filter.feed("done"), "done");
        assert_eq!(filter.flush().unwrap(), "");
    }

    #[test]
    fn splits_on_character_boundaries() {
        // Multi-byte text before a partial tag must not be split mid-character.
        assert_eq!(run(&["译文<thi", "nk>隐藏</think>结果"]), "译文结果");
    }
}
