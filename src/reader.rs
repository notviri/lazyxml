use crate::{Error, event::Event};
use memchr::memchr;
use std::borrow::Cow;

pub struct Reader<'src> {
    state: ReaderState,
    source: Cow<'src, [u8]>,

    trim: bool,
}

enum ReaderState {
    /// The reader isn't particularly on anything. It's looking for text or tags.
    Searching,

    /// The reader is on top of a tag's opening angle bracket (`<`).
    LocatedTag,
}

impl<'src> Reader<'src> {
    // ------------------
    // -- Constructors --
    // ------------------

    #[inline]
    pub fn new(xml: &'src [u8]) -> Self {
        Self::_new(Cow::Borrowed(xml))
    }

    #[inline]
    pub fn from_owned(xml: impl Into<Vec<u8>>) -> Reader<'static> {
        Reader::_new(Cow::Owned(xml.into()))
    }

    fn _new(xml: Cow<'src, [u8]>) -> Self {
        Self {
            state: ReaderState::Searching,
            source: xml,

            trim: true,
        }
    }

    // -----------------------------
    // -- Builder style functions --
    // -----------------------------

    pub fn trim_whitespace(&mut self, trim: bool) -> &mut Self {
        self.trim = trim;
        self
    }

    // -----------------------
    // -- Parsing internals --
    // -----------------------

    pub fn next(&mut self) -> Result<Event<'src>, Error> {
        todo!()
    }
}
