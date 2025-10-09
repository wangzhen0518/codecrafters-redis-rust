pub(crate) fn ascii_to_number(ascii_bytes: &[u8]) -> usize {
    let mut number = 0;
    for c in ascii_bytes {
        number = number * 10 + (c - b'0') as usize;
    }
    number
}

pub(crate) fn number_to_ascii(mut number: usize) -> Vec<u8> {
    let mut ascii_bytes = vec![];
    while number > 0 {
        ascii_bytes.push((number % 10) as u8 + b'0');
        number /= 10;
    }
    ascii_bytes.reverse();
    ascii_bytes
}

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
