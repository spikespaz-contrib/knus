use chumsky::prelude::*;
use chumsky::Error;

use crate::ast::{Literal, TypeName, Node, Value, Integer, Decimal, Radix};
use crate::ast::{SpannedName, SpannedChildren, Document};
use crate::traits::Span;

pub fn newline<E: Error<char>>() -> impl Parser<char, (), Error=E> {
    just('\r')
        .or_not()
        .ignore_then(just('\n'))
        .or(just('\r')) // Carriage return
        .or(just('\x0C')) // Form feed
        .or(just('\u{0085}')) // Next line
        .or(just('\u{2028}')) // Line separator
        .or(just('\u{2029}')) // Paragraph separator
        .ignored()
}

pub fn ws_char<E: Error<char>>() -> impl Parser<char, (), Error=E> {
    filter(|c| matches!(c,
        '\t' | ' ' | '\u{00a0}' | '\u{1680}' |
        '\u{2000}'..='\u{200A}' |
        '\u{202F}' | '\u{205F}' | '\u{3000}' |
        '\u{FEFF}'
    ))
    .ignored()
}

pub fn ws<E: Error<char>>() -> impl Parser<char, (), Error=E> {
    ws_char().repeated().at_least(1).ignored()
}

pub fn comment<E: Error<char>>() -> impl Parser<char, (), Error=E> {
    just("//").then(take_until(newline().or(end()))).ignored()
}

pub fn ml_comment<E: Error<char> + 'static>() -> impl Parser<char, (), Error=E>
{
    recursive(|comment| {
        choice((
            comment,
            none_of('*').ignored(),
            just('*').then_ignore(none_of('/').rewind()).ignored(),
        )).repeated().ignored()
        .delimited_by(just("/*"), just("*/"))
    })
}

/*
fn parser<S: Span>() -> impl Parser<char, Document<S>, Error=Simple<char>> {
    todo!()
}
*/

#[cfg(test)]
mod test {
    use chumsky::prelude::*;
    use super::{ws, comment, ml_comment};

    fn parse<'x, P, T>(p: P, text: &'x str) -> Result<T, String>
        where P: Parser<char, T, Error=Simple<char>>
    {
        let (data, errs) = p.then_ignore(end()).parse_recovery(text);
        if let Some(data) = data {
            return Ok(data);
        }
        Err(errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n"))
    }

    #[test]
    fn parse_ws() {
        parse(ws(), "   ").unwrap();
        parse(ws(), "text").unwrap_err();
    }

    #[test]
    fn parse_comments() {
        parse(comment(), "//hello").unwrap();
        parse(comment(), "//hello\n").unwrap();
        parse(ml_comment(), "/*nothing*/").unwrap();
        parse(ml_comment(), "/*nothing**/").unwrap();
        parse(ml_comment(), "/*no*thing*/").unwrap();
        parse(ml_comment(), "/*no/**/thing*/").unwrap();
        parse(ml_comment(), "/*no/*/**/*/thing*/").unwrap();
        parse(ws().then(comment()), "   // hello").unwrap();
        parse(ws().then(comment()).then(ws()).then(comment()),
              "   // hello\n   //world").unwrap();
    }
}
