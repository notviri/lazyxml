#[derive(Debug)]
pub enum Event<'src> {
    OpenTag(Tag<'src>),
    CloseTag(Tag<'src>),
    EmptyTag(Tag<'src>),

    Text(Text<'src>),

    Eof,
}

#[derive(Debug)]
pub struct Tag<'src> {
    content: &'src [u8],
    name: &'src [u8],
}

#[derive(Debug)]
pub struct Text<'src> {
    content: &'src [u8],
}

impl<'src> Tag<'src> {
    pub(crate) const fn new(name: &'src [u8], content: &'src [u8]) -> Self {
        Self { content, name }
    }
}

impl<'src> Text<'src> {
    #[inline]
    pub(crate) const fn new(content: &'src [u8]) -> Self {
        Self { content }
    }
}
