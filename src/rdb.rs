use crate::utils::{ascii_to_number, number_to_ascii};

const VERSION_LENGTH: usize = 4;

struct RdbFile {
    rdb_version: u8,
    redis_version: String,
}

impl RdbFile {
    pub fn load(bytes_content: &[u8]) -> RdbFile {
        // head "REDIS0012"
        // &bytes_content[..5]; // "REDIS"
        let rdb_version = ascii_to_number(&bytes_content[5..9]) as u8; // version number

        let mut rdb = RdbFile {
            rdb_version,
            redis_version: String::new(),
        };

        // metadata
        // &bytes_content[9..10]; // 0xFA
        let mut index = 10;
        while bytes_content[index] != 0xFE {
            let attr_name_length = bytes_content[index] as usize;
            let attr_name = str::from_utf8(&bytes_content[index + 1..index + 1 + attr_name_length])
                .expect("Invalid UTF-8")
                .to_lowercase();
            index += 1 + attr_name_length;

            let attr_value_length = bytes_content[index] as usize;
            let attr_value =
                str::from_utf8(&bytes_content[index + 1..index + 1 + attr_value_length])
                    .expect("Invalid UTF-8")
                    .to_string();
            index += 1 + attr_value_length;

            match attr_name.as_str() {
                "redis-ver" => rdb.redis_version = attr_value,
                _ => println!("attr_name: {}\nattr_value:{}\n\n", attr_name, attr_value),
            }
        }

        rdb
    }

    pub fn dump(&self) -> Vec<u8> {
        // head
        let mut content = b"REDIS".to_vec();
        let mut version = number_to_ascii(self.rdb_version as usize);
        let pad_len = VERSION_LENGTH - version.len();
        for _ in 0..pad_len {
            version.insert(0, b'0');
        }
        content.extend(version);

        // metadata
        content.push(0xFA);
        // redis-ver
        content.push(0x09);
        content.extend(b"redis-ver");
        content.push(self.redis_version.len() as u8);
        content.extend(self.redis_version.as_bytes());

        content
    }
}
