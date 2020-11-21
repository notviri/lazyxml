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

#[derive(Debug)]
pub struct Text<'xml, T: ?Sized> {
    content: &'xml T,
}

impl<'xml, T: ?Sized> Tag<'xml, T> {
    pub(crate) const fn new(name: &'xml T, content: &'xml T) -> Self {
        Self { content, name }
    }
}

impl<'xml, T: ?Sized> Text<'xml, T> {
    #[inline]
    pub(crate) const fn new(content: &'xml T) -> Self {
        Self { content }
    }
}
