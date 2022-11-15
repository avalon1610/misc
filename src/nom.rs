use anyhow::{bail, Result};
use nom::{error::ErrorKind, Err};
use std::fmt::Display;

trait ParseError<T> {
    fn streaming_context<C>(self, c: C) -> Result<Option<T>>
    where
        C: Display + Send + Sync + 'static;

    fn complete_context<C>(self, c: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static;
}

impl<T, D> ParseError<T> for Result<T, nom::Err<(D, ErrorKind)>>
where
    D: std::fmt::Debug,
{
    fn streaming_context<C>(self, c: C) -> Result<Option<T>>
    where
        C: Display + Send + Sync + 'static,
    {
        Ok(match self {
            Ok(a) => Some(a),
            Err(e) => match e {
                Err::Incomplete(_) => None,
                Err::Error(e) | Err::Failure(e) => {
                    bail!("{}: {:02x?}", c, e);
                }
            },
        })
    }

    fn complete_context<C>(self, c: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
    {
        Ok(match self {
            Ok(a) => a,
            Err(e) => bail!("{}: {:02x?}", c, e),
        })
    }
}
