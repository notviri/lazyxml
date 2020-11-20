use crate::{
    event::{Event, Tag, Text},
    Error,
};
use memchr::memchr;

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

pub struct Reader<'src> {
    // State
    state: ReaderState,
    source: &'src [u8],
    source_pos: usize,

    // Settings
    trim: bool,
}

enum ReaderState {
    /// The reader isn't particularly on anything. It's looking for text or tags.
    Searching,

    /// The reader is on top of a tag's opening angle bracket (`<`).
    LocatedTag,

    /// The source has reached end of file.
    End,
}

impl<'src> Reader<'src> {
    pub const fn new(xml: &'src [u8]) -> Self {
        Self {
            state: ReaderState::Searching,
            source: xml,
            source_pos: 0,

            trim: true,
        }
    }

    /// Enables or disables trimming whitespace in [`Text`] events.
    /// This property is dynamic and can be turned on and off while parsing.
    pub fn trim_whitespace(&mut self, trim: bool) -> &mut Self {
        self.trim = trim;
        self
    }

    pub fn next(&mut self) -> Result<Event<'src>, Error> {
        match self.state {
            ReaderState::Searching => self.next_search(),
            ReaderState::LocatedTag => self.next_tag(),
            ReaderState::End => Ok(Event::Eof),
        }
    }

    fn next_search(&mut self) -> Result<Event<'src>, Error> {
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
            Ok(Event::Text(Text::new(text)))
        } else {
            self.next()
        }
    }

    fn next_tag(&mut self) -> Result<Event<'src>, Error> {
        let source = sl(self.source, self.source_pos);
        match source.get(0).ok_or_else(|| Error::UnexpectedEof(self.source_pos))? {
            b'!' => todo!("bang"),
            b'?' => todo!("pi"),

            // Standard Tags - Start / Empty / End
            first @ _ => {
                let is_closing_tag = *first == b'/';
                match memchr(b'>', source) {
                    Some(idx) => {
                        let inner = sl_end(source, idx);

                        // Separate head & tail (tag name, attributes chunk).
                        // (head, tail) of `<Name a="1"/>` is <[Name] [a="1"]/>
                        // (head, tail) of `<Name />` is <[Name] []/>
                        // (head, tail) of `<Name/>` is <[Name][]/>
                        let (mut head, mut tail) = match inner.iter().position(|&ch| ch <= b' ') {
                            Some(ws) => (sl_end(inner, ws), sl(inner, ws + 1)), // todo chk
                            None => (inner, &[][..]),
                        };

                        // Trim `/` of `/>` in empty tags.
                        let is_empty_tag = inner.last().map(|&ch| ch == b'/').unwrap_or(false);
                        if is_empty_tag {
                            // Note: Yes, this permits `</Name/>` on purpose as a closing tag.
                            // You *could* fix that with checking `is_closing_tag`.
                            if tail.is_empty() {
                                head = &head[..head.len() - 1]; // TODO: can this fail
                            } else {
                                tail = &tail[..tail.len() - 1]; // TODO: can this fail
                            }
                        }

                        // Trim `/` of `</` in closing tags.
                        if is_closing_tag {
                            if head.is_empty() {
                                return Err(Error::InvalidName(self.source_pos - 1)) // `</>`
                            } else {
                                head = &head[1..]; // todo #232
                            }
                        }

                        // Yield tag if name is valid.
                        if is_valid_tag_name(head) {
                            self.source_pos += idx + 1;
                            self.state = ReaderState::Searching;
                            if is_closing_tag {
                                Ok(Event::CloseTag(Tag::new(head, tail)))
                            } else if is_empty_tag {
                                Ok(Event::EmptyTag(Tag::new(head, tail)))
                            } else {
                                Ok(Event::OpenTag(Tag::new(head, tail)))
                            }
                        } else {
                            Err(Error::InvalidName(self.source_pos - 1))
                        }
                    },
                    None => Err(Error::UnexpectedEof(self.source_pos - 1)),
                }
            },
        }
    }
}
