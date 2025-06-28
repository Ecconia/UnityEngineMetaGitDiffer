use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;
use git2::{Oid, Repository};

// Unity Unique Identifier (lel)
#[derive(Copy, Clone)]
#[derive(Hash, Eq, PartialEq)]
#[derive(Ord, PartialOrd)]
pub struct Uuid {
	// Source example: 63079bf56d891f040a461867b5dc65cb
	// Single digit: 1 digit = 16 states = 4 bits => 2 digits/byte
	// Size: 32 digits / 2 digits/byte => 16 bytes
	hash_bytes: [u8; 16],
}

impl Display for Uuid {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let hex : String = self.hash_bytes.iter()
			.map(|b| format!("{b:02x}"))
			.collect();
		write!(f, "{hex}")
	}
}

impl Uuid {
	fn from(input: &str) -> Option<Self> {
		if input.len() != 32 {
			return None;
		}
		let mut bytes = [0u8; 16];
		for (index, item) in bytes.iter_mut().enumerate() {
			let input_index = index << 1;
			*item = match u8::from_str_radix(&input[input_index..(input_index + 1)], 16) {
				Ok(v) => v,
				Err(_) => return None,
			}
		}
		Some(Self {
			hash_bytes: bytes,
		})
	}
	
	pub fn from_disk_or_panic(path: &Path) -> Uuid {
		let text = fs::read_to_string(path).unwrap();
		let uuid_text = Self::from_meta_content(&text).unwrap_or_else(|| panic!("Did not find UUID for path {}", path.display()));
		Uuid::from(&uuid_text).unwrap_or_else(|| panic!("Could not convert UUID '{uuid_text}' in file '{}'", path.display()))
	}
	
	pub fn from_blob_or_panic(repo: &Repository, hash: Oid) -> Uuid {
		let blob = repo.find_blob(hash).unwrap();
		let text = String::from_utf8(blob.content().to_owned()).unwrap();
		let uuid_text = Self::from_meta_content(&text).unwrap_or_else(|| panic!("Did not find UUID for blob {hash}"));
		Uuid::from(&uuid_text).unwrap_or_else(|| panic!("Could not convert UUID '{uuid_text}' in blob {hash}"))
	}
	
	fn from_meta_content(text: &str) -> Option<&str> {
		for line in text.lines() {
			// Technically Unity only ever puts one space into this line (after the colon), but let the code handle a few more spaces:
			if let Some(uid) = line.strip_prefix("guid:") {
				return Some(uid.trim());
			}
		}
		None
	}
}
