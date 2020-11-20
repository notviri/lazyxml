pub mod event;

mod reader;
pub use reader::Reader;

#[derive(Debug)]
pub enum Error {
    /// Tag at (offset) is empty or has an invalid name.
    ///
    /// Examples: `<>`, `< >`, `</>`, `<//>`, `<///>`, `<0Name>`, `<.Name>`, etc.
    InvalidName(usize),

    /// Unexpected end of file was met while reading a tag or attribute.
    ///
    /// Attribute checks are only done by attribute iterators.
    ///
    /// Examples: `<`, `<Name`, `<Name a="1`, `<Name a="1"`.
    UnexpectedEof,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
