use std::fmt::Display;

use bytes::Bytes;

pub fn config_logger() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_file(true)
        .with_line_number(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        // .pretty()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set global subscriber");
}

#[derive(Debug)]
pub enum BytesInStr<'a> {
    Str(&'a str),
    Chars(Vec<char>),
}

impl<'a> BytesInStr<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        if let Ok(s) = str::from_utf8(bytes) {
            BytesInStr::Str(s)
        } else {
            let chars = bytes.iter().map(|c| char::from(*c)).collect();
            BytesInStr::Chars(chars)
        }
    }

    pub fn from_bytes_array(array: &'a [Bytes]) -> Vec<Self> {
        array
            .iter()
            .map(|bytes| {
                if let Ok(s) = str::from_utf8(bytes.as_ref()) {
                    BytesInStr::Str(s)
                } else {
                    let chars = bytes.iter().map(|c| char::from(*c)).collect();
                    BytesInStr::Chars(chars)
                }
            })
            .collect()
    }
}

impl<'a> Display for BytesInStr<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BytesInStr::Str(s) => write!(f, "{:?}", s),
            BytesInStr::Chars(items) => write!(f, "{:?}", items),
        }
    }
}

// impl<'a> Display for Vec<BytesInStr<'a>> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let mut is_first = true;
//         write!(f, "[")?;
//         for bytes in self {
//             if is_first {
//                 is_first = false;
//                 write!(f, "{}", bytes)?;
//             } else {
//                 write!(f, ", {}", bytes)?;
//             }
//         }
//         write!(f, "]")?;
//         Ok(())
//     }
// }
