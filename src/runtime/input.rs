/// An indexable buffer of bytes that can be parsed.
///
/// The parser does not perform any internal buffering on top of this, so implementations should be
/// as performant as possible. A default implementation exists for `[u8]`. No default implementation
/// exists for `str` since it hides the implicit reliance on UTF-8. Use [as_bytes](str::as_bytes) if
/// you want to parse a `str`.
///
/// # Safety
///
/// An incorrect implementation may cause undefined behavior if parsed.
pub unsafe trait Input {
    /// Gets a byte at a particular index if the index is in bounds.
    ///
    /// This must return `Some` if `position < self.len()` and must return `None` if
    /// `position >= self.len()`. The byte at any given index must be constant within a parse.
    fn get(&self, position: u32) -> Option<u8>;

    /// Determines the length of the input.
    ///
    /// This must be constant within a parse.
    fn len(&self) -> u32;
}

unsafe impl Input for [u8] {
    fn get(&self, position: u32) -> Option<u8> {
        self.get(position as usize).copied()
    }

    fn len(&self) -> u32 {
        self.len() as u32
    }
}
