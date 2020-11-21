use crate::{
    event::{Event, Tag, Text},
    Error,
};
use memchr::memchr;
use std::mem;

static IS_INVALID_TAG_NAME_START: [bool; 256] = lut_invalid_tag_name_start();
const fn lut_invalid_tag_name_start() -> [bool; 256] {
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

#[inline]
fn is_valid_tag_name(name: &[u8]) -> bool {
    match name.first().copied() {
        Some(x) => IS_INVALID_TAG_NAME_START[x as usize], // no bounds check (u8 <= sizeof [])
        None => false, // empty, somehow
    }
}

// SAFETY: We only put trusted indices returned by the STL / `memchr` (crate) in here.
// These only exist as mini functions to improve code readability.
#[inline]
fn sl(s: &[u8], x: usize) -> &[u8] {
    unsafe { s.get_unchecked(x..) }
}
#[inline]
fn sl_end(s: &[u8], x: usize) -> &[u8] {
    unsafe { s.get_unchecked(..x) }
}

fn trim(text: &[u8]) -> &[u8] {
    text.iter()
        .position(|&ch| ch > b' ')
        .and_then(|l| text.iter().rposition(|&ch| ch > b' ').map(|r| (l, r)))
        .and_then(|(l, r)| text.get(l..=r))
        .unwrap_or(b"")
}

pub struct Reader<'xml, T: ?Sized> {
    // State
    state: ReaderState,
    source: &'xml T,
    source_pos: usize,

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

impl<'xml, T> Reader<'xml, T> {
    /// Constructs a new [`Reader`] from ASCII-compatible XML bytes.
    pub const fn new(xml: &'xml [u8]) -> Reader<'xml, [u8]> {
        Reader {
            state: ReaderState::Searching,
            source: xml,
            source_pos: 0,

            trim: true,
        }
    }

    /// Constructs a new [`Reader`] from a UTF-8 string.
    pub const fn from_str(xml: &'xml str) -> Reader<'xml, str> {
        Reader {
            state: ReaderState::Searching,
            source: xml,
            source_pos: 0,

            trim: true,
        }
    }

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
    pub fn offset(&self) -> usize {
        self.source_pos
    }
}

impl<'xml> Reader<'xml, [u8]> {
    fn next_search(&mut self) -> Option<Result<Event<'xml, [u8]>, Error>> {
        let source = sl(self.source, self.source_pos);
        let mut text = match memchr(b'<', source) {
            Some(idx) => {
                // We move 1 byte past '<' as we know that's what it is.
                // This makes next access be worst-case &[] (safe).
                self.source_pos += idx + 1;
                self.state = ReaderState::LocatedTag;
                sl_end(source, idx)
            },
            None => {
                self.state = ReaderState::End;
                source
            },
        };
        if self.trim {
            text = trim(text);
        }
        if !text.is_empty() {
            Some(Ok(Event::Text(Text::new(text))))
        } else {
            self.next()
        }
    }

    fn next_tag(&mut self) -> Option<Result<Event<'xml, [u8]>, Error>> {
        let source = sl(self.source, self.source_pos);
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
                        let inner = sl_end(source, idx);

                        // Separate head & tail (tag name, attributes chunk).
                        // (head, tail) of `<Name a="1"/>` is <[Name] [a="1"]/>
                        // (head, tail) of `<Name />` is <[Name] []/>
                        // (head, tail) of `<Name/>` is <[Name][]/>
                        let (mut head, mut tail) = match inner.iter().position(|&ch| ch <= b' ') {
                            Some(space) => (sl_end(inner, space), sl(inner, space + 1)),
                            None => (inner, &[][..]),
                        };

                        // Trim `/` of `/>` in empty tags.
                        let is_empty_tag = inner.last().map(|&ch| ch == b'/').unwrap_or(false);
                        if is_empty_tag {
                            // Note: Yes, this permits `</Name/>` on purpose as a closing tag.
                            // You *could* fix that with checking `is_closing_tag`.
                            if tail.is_empty() {
                                head = sl(head, head.len() - 1);
                            } else {
                                tail = sl(tail, tail.len() - 1);
                            }
                        }

                        // Trim `/` of `</` in closing tags.
                        if is_closing_tag {
                            if head.is_empty() {
                                // A strange case of `</>` would lead here.
                                return Some(Err(Error::InvalidName(self.source_pos - 1)))
                            } else {
                                head = sl(head, 1);
                            }
                        }

                        // Yield tag if name is valid.
                        if is_valid_tag_name(head) {
                            self.source_pos += idx + 1;
                            self.state = ReaderState::Searching;
                            if is_closing_tag {
                                Some(Ok(Event::CloseTag(Tag::new(head, tail))))
                            } else if is_empty_tag {
                                Some(Ok(Event::EmptyTag(Tag::new(head, tail))))
                            } else {
                                Some(Ok(Event::OpenTag(Tag::new(head, tail))))
                            }
                        } else {
                            Some(Err(Error::InvalidName(self.source_pos - 1)))
                        }
                    },
                    None => Some(Err(Error::UnexpectedEof)),
                }
            },
        }
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
