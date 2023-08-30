use std::iter::Peekable;

const CR_CHAR: u8 = b'\r';
const LF_CHAR: u8 = b'\n';

/// Implements the ability to drop `\r\n` byte pairs from an iterator, converting each instance to a single `\n`.
pub struct CRLFToLF<I>
where
    I: Iterator<Item = u8>,
{
    iter: Peekable<I>,
}

impl<I> Iterator for CRLFToLF<I>
where
    I: Iterator<Item = u8>,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.iter.next()?;
        if current == CR_CHAR && self.iter.peek() == Some(&LF_CHAR) {
            self.next()
        } else {
            Some(current)
        }
    }
}

impl<I> CRLFToLF<I>
where
    I: Iterator<Item = u8>,
{
    fn new(iter: I) -> Self {
        Self {
            iter: iter.peekable(),
        }
    }
}

pub trait ConvertCRLFToLF {
    /// Drop `\r\n` byte pairs from an iterator, converting each instance to a single `\n`.
    fn convert_crlf_lf(self) -> CRLFToLF<Self>
    where
        Self: Sized,
        Self: Iterator<Item = u8>;
}

impl<I> ConvertCRLFToLF for I
where
    I: Iterator<Item = u8>,
{
    fn convert_crlf_lf(self) -> CRLFToLF<Self> {
        CRLFToLF::new(self)
    }
}
