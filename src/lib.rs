pub mod event;

mod reader;
pub use reader::Reader;

#[derive(Debug)]
pub enum Error {
    /// Tag at (offset) is empty or has an invalid name.
    ///
    /// Examples: `<>`, `< >`, `</>`, `<//>`, `<///>`, `<0Name>`, `<.Name>`, etc.
    InvalidName(usize),

    /// Unexpected end of file was met at (offset) while reading.
    ///
    /// Examples: `<`, `<Name`, `<Name a="1"`.
    UnexpectedEof(usize),
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
