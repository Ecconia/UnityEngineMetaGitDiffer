use git2::{Delta, Diff, DiffDelta, DiffOptions, Oid, Repository};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::{env, fs};
use ecc_ansi_lib::ansi;

// Unity Unique Identifier (lel)
#[derive(Copy, Clone)]
#[derive(Hash, Eq, PartialEq)]
#[derive(Ord, PartialOrd)]
struct Uuid {
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
	
	fn from_disk_or_panic(path: &Path) -> Uuid {
		let text = fs::read_to_string(path).unwrap();
		let uuid_text = Self::from_meta_content(&text).unwrap_or_else(|| panic!("Did not find UUID for path {}", path.display()));
		Uuid::from(&uuid_text).unwrap_or_else(|| panic!("Could not convert UUID '{uuid_text}' in file '{}'", path.display()))
	}
	
	fn from_blob_or_panic(repo: &Repository, hash: Oid) -> Uuid {
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

#[derive(Default)]
#[derive(Clone)]
struct UuidStorageEntry {
	added: Vec<PathBuf>,
	removed: Vec<PathBuf>,
}

#[derive(Default)]
struct UuidStorage {
	lookup: HashMap<Uuid, UuidStorageEntry>
}

impl UuidStorage {
	fn get_node(&mut self, uuid: Uuid) -> &mut UuidStorageEntry {
		self.lookup.entry(uuid).or_default()
	}
	
	fn added(&mut self, uuid: Uuid, path: PathBuf) {
		self.get_node(uuid).added.push(path);
	}
	
	fn removed(&mut self, uuid: Uuid, path: PathBuf) {
		self.get_node(uuid).removed.push(path);
	}
	
	fn debug_print(&self) {
		let mut list: Vec<_> = self.lookup.iter().collect();
		// HashMaps are ordered with a random seed - sort to ensure consistent output order.
		list.sort_by_key(|item| item.0);
		
		for (uuid, storage) in list.into_iter() {
			println!("{uuid}:");
			for removed in storage.removed.iter() {
				println!(ansi!("  «lr»{}«»"), removed.display());
			}
			for added in storage.added.iter() {
				println!(ansi!("  «lg»{}«»"), added.display());
			}
		}
	}
}

fn main() {
	env::set_current_dir("../../SourceCode").unwrap();
	println!("Path: {}", env::current_dir().unwrap().display());
	
	let repo = match Repository::open(".") {
		Ok(repo) => repo,
		Err(e) => panic!("failed to open: {e}"),
	};
	let diff = create_diff_from_arguments(&repo);
	let diffs = gather_filtered_deltas_from_diff(&diff);
	let mut uuid_storage = UuidStorage::default();
	
	
	println!("Unstaged: {}", diffs.len());
	for delta in diffs.iter() {
		if delta.new_file().path().is_none() || delta.old_file().path().is_none() || delta.new_file().path().unwrap() != delta.old_file().path().unwrap() {
			panic!("The path of the old/new file did not match or one/both had not been set: {:?} ||| {:?}", delta.old_file(), delta.new_file());
		}
		
		let path = delta.old_file().path().unwrap().to_path_buf();
		match delta.status() {
			Delta::Untracked => {
				let uuid = Uuid::from_disk_or_panic(&path);
				// println!(ansi!("«lg»++ {}«» {}"), &path.display(), uuid);
				uuid_storage.added(uuid, path);
			}
			Delta::Added => {
				let hash = delta.new_file().id();
				let uuid = Uuid::from_blob_or_panic(&repo, hash);
				// println!(ansi!("«lg»++ {}«» {}"), &path.display(), uuid);
				uuid_storage.added(uuid, path);
			}
			Delta::Deleted => {
				let hash = delta.old_file().id();
				let uuid = Uuid::from_blob_or_panic(&repo, hash);
				// println!(ansi!("«lr»-- {}«» {}"), &path.display(), uuid);
				uuid_storage.removed(uuid, path);
			}
			Delta::Modified => {
				let uuid_from = Uuid::from_blob_or_panic(&repo, delta.old_file().id());
				let uuid_to = Uuid::from_blob_or_panic(&repo, delta.new_file().id());
				if uuid_from != uuid_to {
					// println!("   {} {uuid_from} => {uuid_to}", &path.display());
					uuid_storage.removed(uuid_from, path.clone());
					uuid_storage.added(uuid_to, path);
				}
			}
			_ => {
				panic!("Cannot yet handle diff delta type of {:?}", delta.status());
			}
		}
	}
	
	uuid_storage.debug_print();
}

fn gather_filtered_deltas_from_diff<'a>(diff: &'a Diff<'a>) -> Vec<DiffDelta<'a>> {
	diff.deltas().filter(|delta| {
		let old = delta.old_file().path();
		let new = delta.new_file().path();
		old.is_some() && old.unwrap().to_str().unwrap().ends_with(".meta")
			|| new.is_some() && new.unwrap().to_str().unwrap().ends_with(".meta")
	}).collect()
}

fn create_diff_from_arguments(repo: &Repository) -> Diff {
	let mut diff_opts = DiffOptions::new();
	diff_opts.include_untracked(true);
	
	// let a = repo.find_commit_by_prefix("64c3f3a87132840c83541b1d10a6cff031fd7800").unwrap().tree().unwrap();
	// let b = repo.find_commit_by_prefix("77cd7307e14adc9fb18c6473e65f05093bb3e9f4").unwrap().tree().unwrap();
	// repo.diff_tree_to_tree(Some(&a), Some(&b), None).unwrap()
	
	let head_commit = repo.head().unwrap().peel_to_commit().unwrap().tree().unwrap();
	repo.diff_tree_to_workdir_with_index(Some(&head_commit), Some(&mut diff_opts)).unwrap()
}
