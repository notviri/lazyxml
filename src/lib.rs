use memchr::{memchr, memchr2};
use std::mem;

#[derive(Debug)]
pub enum Error {
    /// Tag at (offset) is empty or has an invalid name.
    ///
    /// Examples: `<>`, `< >`, `</>`, `<//>`, `<///>`, `<0Name>`, `<.Name>`, etc.
    InvalidName(usize),

    /// Attribute is malformed.
    ///
    /// Offset is relative to the [`Tag`]'s content chunk.
    ///
    /// Examples: `<Name a>`, `<Name a= >`, `<Name ="1">`, `<Name a=1>`.
    InvalidAttribute(usize),

    /// Unexpected end of file was met while reading a tag or attribute.
    ///
    /// Attribute checks are only done by attribute iterators.
    ///
    /// Examples: `<`, `<Name`, `<Name a`, `<Name a=`, `<Name a="1`, `<Name a="1"`.
    UnexpectedEof,
}

#[derive(Debug)]
pub enum Event<'xml, T: ?Sized> {
    OpenTag(Tag<'xml, T>),
    CloseTag(Tag<'xml, T>),
    EmptyTag(Tag<'xml, T>),

    Text(Text<'xml, T>),
}

#[derive(Debug)]
pub struct Tag<'xml, T: ?Sized> {
    content: &'xml T,
    name: &'xml T,
}

pub struct AttributeIter<'xml, T: ?Sized> {
    content: &'xml T,
    offset: usize,
}

#[derive(Debug)]
pub struct Attribute<'xml, T: ?Sized> {
    pub key: &'xml T,
    pub value: &'xml T,
}

#[derive(Debug)]
pub struct Text<'xml, T: ?Sized> {
    content: &'xml T,
}

pub struct Reader<'xml, T: ?Sized> {
    // State
    state: ReaderState,
    source: &'xml T,
    offset: usize,

    // Settings
    trim: bool,
}

enum ReaderState {
    /// The reader isn't particularly on anything. It's looking for text or tags.
    Searching,

    /// The reader is on top of a tag (one past the opening angle bracket `<`).
    LocatedTag,

    /// The source has reached end of file.
    End,
}

static IS_VALID_NAME_START: [bool; 256] = lut_name_start_chars();
const fn lut_name_start_chars() -> [bool; 256] {
    let mut arr = [true; 256];
    let mut i = 0;
    while i < 256 {
        arr[i] = match i as u8 {
            0x00..=b' ' => false,
            b'!'..=b'9' => false,
            b':'..=b'@' => false,
            b'['..=b'`' => false,
            b'{'..=0x7F => false,
            _ => true,
        };
        i += 1;
    }
    arr
}

// SAFETY: We only put trusted indices returned by the STL / `memchr` (crate) in here.
// These only exist as mini functions to improve code readability.
#[inline]
fn sl(s: &[u8], x: usize) -> &[u8] {
    unsafe { s.get_unchecked(x..) }
}
#[inline]
fn sl_to(s: &[u8], x: usize) -> &[u8] {
    unsafe { s.get_unchecked(..x) }
}

fn trim_whitespace(text: &[u8]) -> &[u8] {
    text.iter()
        .position(|&ch| ch > b' ')
        .and_then(|l| text.iter().rposition(|&ch| ch > b' ').map(|r| (l, r)))
        .and_then(|(l, r)| text.get(l..=r))
        .unwrap_or(b"")
}

#[inline]
fn is_valid_tag_name(name: &[u8]) -> bool {
    match name.first().copied() {
        // no bounds check is done with [] since the array is at least u8::max_value()
        Some(x) => IS_VALID_NAME_START[x as usize],
        None => false, // empty, somehow
    }
}

impl<'xml, T: ?Sized> Tag<'xml, T> {
    pub(crate) const fn new(name: &'xml T, content: &'xml T) -> Self {
        Self { content, name }
    }

    /// Returns an iterator over the tag's attributes, if any.
    pub const fn attributes(&self) -> AttributeIter<'xml, T> {
        AttributeIter {
            content: self.content,
            offset: 0,
        }
    }
}

impl<'xml> Iterator for AttributeIter<'xml, [u8]> {
    type Item = Result<Attribute<'xml, [u8]>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut source = sl(self.content, self.offset);
        let initial_offset = self.offset;

        // Ignore preceding whitespace (happens between attributes too, sometimes*).
        // * The standard actually requires it but we don't care.
        self.offset += source.iter().position(|&ch| ch > b' ')?;
        source = sl(self.content, self.offset);

        // Find `=` key/value separator
        let sep_offset = match memchr(b'=', source) {
            Some(sep) => sep,
            None => return Some(Err(Error::UnexpectedEof)),
        };
        self.offset += sep_offset;

        // Trim whitespace around key so a="1" and a = "1" behave the same
        let key = trim_whitespace(sl_to(source, sep_offset));
        if key.is_empty() {
            return Some(Err(Error::InvalidAttribute(initial_offset)))
        }
        self.offset += 1; // move past `=`

        // Find starting quote, either `'` or `"`.
        // Memchr is not used here because 99.999% it'll be offset 0 (a="1") or 1 (a = "1").
        source = sl(self.content, self.offset);
        let (offset, quote_char) = match source
            .iter()
            .enumerate()
            .find(|&(ix, ch)| *ch == b'"' || *ch == b'\'')
        {
            Some((ix, ch)) => (ix, *ch),
            None => return Some(Err(Error::InvalidAttribute(initial_offset))),
        };
        self.offset += offset + 1; // past the quote
        source = sl(self.content, self.offset);

        // Yield key & value if available.
        match memchr(quote_char, source) {
            Some(end) => {
                let value = sl_to(source, end);
                self.offset += end + 1; // past the closing quote
                Some(Ok(Attribute::new(key, value)))
            },
            None => Some(Err(Error::InvalidAttribute(initial_offset))),
        }
    }
}

impl<'xml> Iterator for AttributeIter<'xml, str> {
    type Item = Result<Attribute<'xml, str>, Error>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: Identical layout, contents, and that's how the standard library does it too.
        unsafe {
            mem::transmute(mem::transmute::<_, &mut AttributeIter<'xml, [u8]>>(self).next())
        }
    }
}

impl<'xml, T: ?Sized> Attribute<'xml, T> {
    pub(crate) const fn new(key: &'xml T, value: &'xml T) -> Self {
        Self { key, value }
    }
}

impl<'xml, T: ?Sized> Text<'xml, T> {
    #[inline]
    pub(crate) const fn new(content: &'xml T) -> Self {
        Self { content }
    }
}


impl<'xml, T> Reader<'xml, T> {
    /// Enables or disables trimming whitespace in [`Text`] events.
    ///
    /// This property is dynamic and can be turned on and off while parsing.
    ///
    /// Defaults to enabled (`true`).
    pub fn trim_whitespace(&mut self, trim: bool) -> &mut Self {
        self.trim = trim;
        self
    }

    /// Gets the byte offset from the start of the input.
    pub const fn offset(&self) -> usize {
        self.offset
    }
}

impl<'xml> Reader<'xml, [u8]> {
    /// Constructs a new [`Reader`] from ASCII-compatible XML bytes.
    pub const fn from_bytes(xml: &'xml [u8]) -> Reader<'xml, [u8]> {
        Reader {
            state: ReaderState::Searching,
            source: xml,
            offset: 0,

            trim: true,
        }
    }

    fn next_search(&mut self) -> Option<Result<Event<'xml, [u8]>, Error>> {
        let source = sl(self.source, self.offset);
        let mut text = match memchr(b'<', source) {
            Some(idx) => {
                // We move 1 byte past '<' as we know that's what it is.
                // This makes next access be worst-case &[] (safe).
                self.offset += idx + 1;
                self.state = ReaderState::LocatedTag;
                sl_to(source, idx)
            },
            None => {
                self.state = ReaderState::End;
                source
            },
        };
        if self.trim {
            text = trim_whitespace(text);
        }
        if !text.is_empty() {
            Some(Ok(Event::Text(Text::new(text))))
        } else {
            self.next()
        }
    }

    fn next_tag(&mut self) -> Option<Result<Event<'xml, [u8]>, Error>> {
        let source = sl(self.source, self.offset);
        let first_char = match source.get(0) {
            Some(ch) => ch,
            None => return Some(Err(Error::UnexpectedEof)),
        };
        match first_char {
            b'!' => todo!("bang"),
            b'?' => todo!("pi"),

            // Standard Tags - Start / Empty / End
            first @ _ => {
                let is_closing_tag = *first == b'/';
                match memchr(b'>', source) {
                    Some(idx) => {
                        // The inner content is the entire slice <[between]> the angle brackets.
                        let inner = sl_to(source, idx);

                        // Separate head & tail (tag name, attributes chunk).
                        // (head, tail) of `<Name a="1"/>` is <[Name] [a="1"]/>
                        // (head, tail) of `<Name />` is <[Name] []/>
                        // (head, tail) of `<Name/>` is <[Name/][]>
                        let (mut head, mut tail) = match inner.iter().position(|&ch| ch <= b' ') {
                            Some(space) => (sl_to(inner, space), sl(inner, space + 1)),
                            None => (inner, &[][..]),
                        };

                        // Trim `/` of `/>` in empty tags.
                        let is_empty_tag = inner.last().map(|&ch| ch == b'/').unwrap_or(false);
                        if is_empty_tag {
                            // Note: Yes, this permits `</Name/>` on purpose as a closing tag.
                            // You *could* fix that with checking `is_closing_tag`.
                            if tail.is_empty() {
                                head = sl_to(head, head.len() - 1);
                            } else {
                                tail = sl_to(tail, tail.len() - 1);
                            }
                        }

                        // Trim `/` of `</` in closing tags.
                        if is_closing_tag {
                            if head.is_empty() {
                                // A strange case of `</>` would lead here.
                                return Some(Err(Error::InvalidName(self.offset - 1)))
                            } else {
                                head = sl(head, 1);
                            }
                        }

                        // Yield tag if name is valid.
                        if is_valid_tag_name(head) {
                            self.offset += idx + 1;
                            self.state = ReaderState::Searching;
                            if is_closing_tag {
                                Some(Ok(Event::CloseTag(Tag::new(head, tail))))
                            } else if is_empty_tag {
                                Some(Ok(Event::EmptyTag(Tag::new(head, tail))))
                            } else {
                                Some(Ok(Event::OpenTag(Tag::new(head, tail))))
                            }
                        } else {
                            Some(Err(Error::InvalidName(self.offset - 1)))
                        }
                    },
                    None => Some(Err(Error::UnexpectedEof)),
                }
            },
        }
    }
}

impl<'xml> Reader<'xml, str> {
    /// Constructs a new [`Reader`] from a UTF-8 string.
    pub const fn from_str(xml: &'xml str) -> Reader<'xml, str> {
        Reader {
            state: ReaderState::Searching,
            source: xml,
            offset: 0,

            trim: true,
        }
    }

    /// Constructs a new [`Reader`] from a UTF-8 string, stripping the BOM if it's present.
    #[inline]
    pub fn from_str_bom(xml: &'xml str) -> Reader<'xml, str> {
        Self::from_str(xml.trim_start_matches('\u{feff}'))
    }
}

impl<'xml> Iterator for Reader<'xml, [u8]> {
    type Item = Result<Event<'xml, [u8]>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            ReaderState::Searching => self.next_search(),
            ReaderState::LocatedTag => self.next_tag(),
            ReaderState::End => None,
        }
    }
}

impl<'xml> Iterator for Reader<'xml, str> {
    type Item = Result<Event<'xml, str>, Error>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: Identical layout, contents, and that's how the standard library does it too.
        unsafe {
            mem::transmute(mem::transmute::<_, &mut Reader<'xml, [u8]>>(self).next())
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
