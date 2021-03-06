# lazyxml

[![Crates.io](https://img.shields.io/crates/v/lazyxml)](https://crates.io/crates/lazyxml)
[![Docs.rs](https://docs.rs/lazyxml/badge.svg)](https://docs.rs/lazyxml)
[![GitHub license](https://img.shields.io/github/license/notviri/lazyxml)](https://github.com/notviri/lazyxml/blob/master/LICENSE.md)

Lightweight, lazy, and ignorant XML parsing. Happily looks past completely invalid data, as long as it logically parses. You should almost definitely not use this, unless:

1) You are looking to be as ***lazy*** as ActionScript's `XMLDocument` class, which is what this crate's purpose for existing is.
2) Your data is already validated, but then you should ask yourself why you're still transporting it in XML of all things.
3) You are a performance freak.

## Usage

```rust
// Example code here.
```

## Valid XML

You might ask what counts as valid, if you aren't here specifically for reason #1 listed above.
Here's a valid XML snippet:

```xml
<Name tag="1" a"'"''"'""'''32'34fdhfjsklflsjeje2!!!!!="e"tag2='

'/>
```

I should probably document why this is OK in case anyone outside of reason #1 becomes interested.
