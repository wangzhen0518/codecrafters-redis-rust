use bytes::{Buf, BufMut, Bytes, BytesMut};
use thiserror::Error;

static SEP_STR: &str = "\r\n";
static SEP_BYTES: &[u8; 2] = b"\r\n";
const SEP_LEN: usize = SEP_BYTES.len();
const MAX_NUMBER_STR_LEN: usize = lexical_core::BUFFER_SIZE;

#[derive(Debug)]
pub enum RespData {
    // #[serde(rename = "_")]
    Null,
    // #[serde(rename = "#")]
    Boolean(bool),
    // #[serde(rename = ":")]
    Integer(i64),
    // #[serde(rename = ",")]
    Double(f64),
    // #[serde(rename = "(")]
    // BigNumber(),
    // #[serde(rename = "+")]
    SimpleString(String), //todo 如何采用 &str
    // #[serde(rename = "-")]
    SimpleError(String), //todo 如何采用 &str
    // #[serde(rename = "$")]
    BulkString(Option<Bytes>),
    // #[serde(rename = "!")]
    BulkError(Bytes),
    // #[serde(rename = "=")]
    // VerbatimString(),
    // #[serde(rename = "*")]
    Array(Vec<RespData>),
    // #[serde(rename = "~")]
    // Set(HashSet<RespValue<'a>>),
    // #[serde(rename = "%")]
    // Map(Map<'a>),
    // #[serde(rename = ">")]
    // Push(Vec<RespData>),
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected end. Current buffer content: {:?}", .0)]
    Eof(Bytes),

    // #[error("Expected boolean: {}",  .0)]
    // ParseBooleanError(#[from] std::str::ParseBoolError),
    #[error("Not a utf8 str: {}",  .0)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Failed to parse an integer: {}",  .0)]
    ParseIntegerError(#[from] std::num::ParseIntError),

    #[error("Failed to parse a float: {}",  .0)]
    ParseFloatError(#[from] std::num::ParseFloatError),

    #[error("Lexical core error: {}",  .0)]
    LexicalCoreError(#[from] lexical_core::Error),

    #[error("Expected length: {}, but got length: {}, the content is \"{:?}\".", .0, .1, .2)]
    LengthMissMatch(usize, usize, Bytes),

    #[error("Expected as least length: {}, but got length: {}, the content is \"{:?}\".", .0, .1, .2)]
    LengthNotEnough(usize, usize, Bytes),

    #[error("Expected seperater \"{:?}\", but got \"{:?}\".", SEP_STR, .0)]
    ExpectedSep(Bytes), //TODO 使用引用

    #[error("Expected boolean, but got \"{:?}\".", .0)]
    ExpectedBoolean(Bytes),

    #[error("Expected a bulk string, but got '{}'.", .0)]
    ExpectedBulkString(u8),

    #[error("Expected a not null bulk string, but got a null bulk string.")]
    ExpectedNotNullBulkString,

    #[error("Expected an array, but got '{}'", .0)]
    ExpectedArray(u8),

    #[error("Unknown RESP type prefix: '{}'", .0)]
    UnknownTypePrefix(u8),
}

type Result<T> = std::result::Result<T, ParseError>;

// ======================================== Parse ========================================
#[inline]
fn check_length(buffer: &BytesMut, length: usize) -> Result<()> {
    if buffer.len() < length {
        return Err(ParseError::Eof(Bytes::copy_from_slice(buffer)));
    };
    Ok(())
}

#[inline]
fn read_u8(buffer: &mut BytesMut) -> Result<u8> {
    check_length(buffer, 1)?;
    Ok(buffer.get_u8())
}

#[inline]
fn split_to(buffer: &mut BytesMut, length: usize) -> Result<BytesMut> {
    check_length(buffer, length)?;
    Ok(buffer.split_to(length))
}

#[inline]
fn expect_sep(buffer: &mut BytesMut) -> Result<()> {
    let sep = split_to(buffer, SEP_LEN)?;
    if sep.as_ref() != SEP_BYTES {
        return Err(ParseError::ExpectedSep(Bytes::from_owner(sep)));
    }
    Ok(())
}

#[inline]
fn expect_bulk_string(buffer: &mut BytesMut) -> Result<()> {
    let c = read_u8(buffer)?;
    if c != b'$' {
        return Err(ParseError::ExpectedBulkString(c));
    }
    Ok(())
}

#[inline]
fn expect_array(buffer: &mut BytesMut) -> Result<()> {
    let c = read_u8(buffer)?;
    if c != b'*' {
        return Err(ParseError::ExpectedArray(c));
    }
    Ok(())
}

fn get_bytes_until_next_sep_pos(buffer: &mut BytesMut) -> Result<(BytesMut, usize)> {
    let pos = buffer
        .array_windows()
        .position(|sep| sep == SEP_BYTES)
        .ok_or(ParseError::Eof(Bytes::copy_from_slice(buffer)))?;
    let data = buffer.split_to(pos);
    buffer.advance(SEP_LEN);
    Ok((data, pos))
}

pub fn parse_null(buffer: &mut BytesMut) -> Result<()> {
    expect_sep(buffer)
}

pub fn parse_boolean(buffer: &mut BytesMut) -> Result<bool> {
    let (data, _) = get_bytes_until_next_sep_pos(buffer)?;
    match data.as_ref() {
        b"true" => Ok(true),
        b"false" => Ok(false),
        _ => Err(ParseError::ExpectedBoolean(Bytes::from_owner(data))),
    }
}

pub fn parse_integer(buffer: &mut BytesMut) -> Result<i64> {
    let (data, _) = get_bytes_until_next_sep_pos(buffer)?;
    let integer = lexical_core::parse(&data)?;
    Ok(integer)
}

pub fn parse_double(buffer: &mut BytesMut) -> Result<f64> {
    let (data, _) = get_bytes_until_next_sep_pos(buffer)?;
    let double = lexical_core::parse(&data)?;
    Ok(double)
}

pub fn parse_simple_string<'a>(buffer: &mut BytesMut) -> Result<String> {
    let (data, _) = get_bytes_until_next_sep_pos(buffer)?;
    let s = str::from_utf8(&data)?.to_string();
    Ok(s)
}

pub fn parse_simple_error(buffer: &mut BytesMut) -> Result<String> {
    let (data, _) = get_bytes_until_next_sep_pos(buffer)?;
    let s = str::from_utf8(&data)?.to_string();
    Ok(s)
}

pub fn parse_bulk_string(buffer: &mut BytesMut) -> Result<Option<Bytes>> {
    let (data, _) = get_bytes_until_next_sep_pos(buffer)?;
    let s_len: i64 = lexical_core::parse(&data)?;
    if s_len == -1 {
        Ok(None)
    } else {
        let data = Bytes::from_owner(split_to(buffer, s_len as usize)?);
        expect_sep(buffer)?;
        Ok(Some(data))
    }
}

pub fn parse_bulk_error(buffer: &mut BytesMut) -> Result<Bytes> {
    let (data, _) = get_bytes_until_next_sep_pos(buffer)?;
    let s_len = lexical_core::parse(&data)?;
    let data = Bytes::from_owner(split_to(buffer, s_len)?);
    expect_sep(buffer)?;
    Ok(data)
}

pub fn parse_array(buffer: &mut BytesMut) -> Result<Vec<RespData>> {
    let (data, _) = get_bytes_until_next_sep_pos(buffer)?;
    let length = lexical_core::parse(&data)?;

    let mut array = Vec::with_capacity(length);
    for _ in 0..length {
        let data = parse_resp(buffer)?;
        array.push(data);
    }

    Ok(array)
}

pub fn parse_resp(buffer: &mut BytesMut) -> Result<RespData> {
    match read_u8(buffer)? {
        b'_' => {
            parse_null(buffer)?;
            Ok(RespData::Null)
        }
        b'#' => Ok(RespData::Boolean(parse_boolean(buffer)?)),
        b':' => Ok(RespData::Integer(parse_integer(buffer)?)),
        b',' => Ok(RespData::Double(parse_double(buffer)?)),
        b'+' => Ok(RespData::SimpleString(parse_simple_string(buffer)?)),
        b'-' => Ok(RespData::SimpleError(parse_simple_error(buffer)?)),
        b'$' => Ok(RespData::BulkString(parse_bulk_string(buffer)?)),
        b'!' => Ok(RespData::BulkError(parse_bulk_error(buffer)?)),
        b'*' => Ok(RespData::Array(parse_array(buffer)?)),
        t => Err(ParseError::UnknownTypePrefix(t)),
    }
}

pub struct ClientRequest {
    pub command: String, //todo 使用 &str
    pub args: Vec<Bytes>,
}

pub fn parse_client_request(buffer: &mut BytesMut) -> Result<ClientRequest> {
    expect_array(buffer)?;
    let (data, _) = get_bytes_until_next_sep_pos(buffer)?;
    let length = lexical_core::parse(&data)?;
    if length == 0 {
        return Err(ParseError::LengthMissMatch(
            1,
            length,
            Bytes::from_owner(data),
        ));
    }

    let mut array = Vec::with_capacity(length);
    for _ in 0..length {
        expect_bulk_string(buffer)?;
        let Some(data) = parse_bulk_string(buffer)? else {
            return Err(ParseError::ExpectedNotNullBulkString);
        };

        array.push(data);
    }

    Ok(ClientRequest {
        command: str::from_utf8(&array[0])?.to_uppercase(),
        args: array[1..].to_vec(),
    })
}

// ======================================== Serialize ========================================
#[inline]
fn lexical_write<N: lexical_core::ToLexical>(n: N, buffer: &mut BytesMut) {
    let length = buffer.len();
    buffer.resize(length + MAX_NUMBER_STR_LEN, 0);
    let bytes_written = lexical_core::write(n, &mut buffer[length..]).len();
    buffer.truncate(length + bytes_written);
}

pub fn serialize_null(buffer: &mut BytesMut) {
    buffer.put_u8(b'_');
    buffer.put(SEP_STR.as_bytes());
}

pub fn serialize_boolean(buffer: &mut BytesMut, boolean: bool) {
    buffer.put_u8(b'#');
    buffer.put(if boolean { "true" } else { "false" }.as_bytes());
    buffer.put(SEP_STR.as_bytes());
}

pub fn serialize_integer(buffer: &mut BytesMut, integer: i64) {
    buffer.put_u8(b':');
    lexical_write(integer, buffer);
    buffer.put(SEP_STR.as_bytes());
}

pub fn serialize_double(buffer: &mut BytesMut, double: f64) {
    buffer.put_u8(b':');
    lexical_write(double, buffer);
    buffer.put(SEP_STR.as_bytes());
}

pub fn serialize_simple_string(buffer: &mut BytesMut, s: &str) {
    buffer.put_u8(b'+');
    buffer.put(s.as_bytes());
    buffer.put(SEP_STR.as_bytes());
}

pub fn serialize_simple_error(buffer: &mut BytesMut, s: &str) {
    buffer.put_u8(b'-');
    buffer.put(s.as_bytes());
    buffer.put(SEP_STR.as_bytes());
}

pub fn serialize_bulk_string(buffer: &mut BytesMut, bytes: &Option<Bytes>) {
    buffer.put_u8(b'$');
    buffer.reserve(MAX_NUMBER_STR_LEN);
    match bytes {
        Some(bytes) => {
            lexical_write(bytes.len(), buffer);
            buffer.put(SEP_STR.as_bytes());

            buffer.put(bytes.clone());
        }
        None => {
            lexical_write(-1, buffer);
        }
    };
    buffer.put(SEP_STR.as_bytes());
}

pub fn serialize_bulk_error(buffer: &mut BytesMut, bytes: &Bytes) {
    buffer.put_u8(b'!');
    lexical_write(bytes.len(), buffer);
    buffer.put(SEP_STR.as_bytes());

    buffer.put(bytes.clone());
    buffer.put(SEP_STR.as_bytes());
}

pub fn serialize_array(buffer: &mut BytesMut, array: &[RespData]) {
    buffer.put_u8(b'*');
    lexical_write(array.len(), buffer);
    buffer.put(SEP_STR.as_bytes());

    for resp in array {
        serialize_resp(buffer, resp);
    }
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse_client_request};
    use bytes::BytesMut;

    #[test]
    fn parse_client_request_returns_eof_for_partial_frame() {
        let mut buffer = BytesMut::from("*2\r\n$4\r\nECHO\r\n$5\r\nHEL");
        let result = parse_client_request(&mut buffer);
        assert!(matches!(result, Err(ParseError::Eof(_))));
    }

    #[test]
    fn parse_client_request_succeeds_after_more_bytes_arrive() {
        let mut buffer = BytesMut::from("*2\r\n$4\r\nECHO\r\n$5\r\nHEL");
        buffer.extend_from_slice(b"LO\r\n");

        let request = parse_client_request(&mut buffer).expect("request should parse");
        assert_eq!(request.command, "ECHO");
        assert_eq!(request.args.len(), 1);
        assert_eq!(request.args[0].as_ref(), b"HELLO");
    }
}

const INITIAL_SIZE: usize = 64;
pub fn serialize_resp(buffer: &mut BytesMut, resp: &RespData) {
    match resp {
        RespData::Null => serialize_null(buffer),
        RespData::Boolean(boolean) => serialize_boolean(buffer, *boolean),
        RespData::Integer(integer) => serialize_integer(buffer, *integer),
        RespData::Double(double) => serialize_double(buffer, *double),
        RespData::SimpleString(s) => serialize_simple_string(buffer, s),
        RespData::SimpleError(s) => serialize_simple_error(buffer, s),
        RespData::BulkString(bytes) => serialize_bulk_string(buffer, bytes),
        RespData::BulkError(bytes) => serialize_bulk_error(buffer, bytes),
        RespData::Array(array) => serialize_array(buffer, array),
    }
}
