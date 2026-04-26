use bytes::Bytes;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("Not a utf8 str: {}",  .0)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Failed to parse an integer: {}",  .0)]
    ParseIntegerError(#[from] std::num::ParseIntError),

    #[error("Failed to parse a float: {}",  .0)]
    ParseFloatError(#[from] std::num::ParseFloatError),

    #[error("Lexical core error: {}",  .0)]
    LexicalCoreError(#[from] lexical_core::Error),

    #[error("`{}` has been set to \"{}\", but now set `{}` to \"{}\" again.", .name, .value, .new_name, .new_value)]
    ValueHasBeenSet {
        name: &'static str,
        value: String,
        new_name: String,
        new_value: String,
    },

    #[error("Invalid argument: \"{}\"", .0)]
    InvalidArgument(String),

    #[error("Expected length: {}, but got length: {}, the content is \"{:?}\".", .0, .1, .2)]
    ExpectLengthEq(usize, usize, Vec<Bytes>),

    #[error("Expected length greater : {}, but got length: {}, the content is \"{:?}\".", .0, .1, .2)]
    ExpectLengthGe(usize, usize, Vec<Bytes>),
}

pub(super) type ParseResult<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Error)]
pub enum ExecError {
    // #[error("Failed to obtain database.")]
    // ObtainDbFailed,
}

pub(super) type ExecResult<T> = std::result::Result<T, ExecError>;
