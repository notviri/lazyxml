mod event;
mod reader;

pub use reader::Reader;

pub enum Error {
    No,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
