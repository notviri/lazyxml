#[derive(Debug)]
pub enum Event<'xml> {
    OpenTag(Tag<'xml>),
    CloseTag(Tag<'xml>),
    EmptyTag(Tag<'xml>),

    Text(Text<'xml>),
}

#[derive(Debug)]
pub struct Tag<'xml> {
    content: &'xml [u8],
    name: &'xml [u8],
}

#[derive(Debug)]
pub struct Text<'xml> {
    content: &'xml [u8],
}

impl<'xml> Tag<'xml> {
    pub(crate) const fn new(name: &'xml [u8], content: &'xml [u8]) -> Self {
        Self { content, name }
    }
}

impl<'xml> Text<'xml> {
    #[inline]
    pub(crate) const fn new(content: &'xml [u8]) -> Self {
        Self { content }
    }
}
