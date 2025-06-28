use std::collections::HashMap;
use std::path::PathBuf;
use ecc_ansi_lib::ansi;
use crate::data::uuid::Uuid;

#[derive(Default)]
#[derive(Clone)]
pub struct UuidStorageEntry {
	added: Vec<PathBuf>,
	removed: Vec<PathBuf>,
}

#[derive(Default)]
pub struct UuidStorage {
	lookup: HashMap<Uuid, UuidStorageEntry>
}

impl UuidStorage {
	fn get_node(&mut self, uuid: Uuid) -> &mut UuidStorageEntry {
		self.lookup.entry(uuid).or_default()
	}
	
	pub fn added(&mut self, uuid: Uuid, path: PathBuf) {
		self.get_node(uuid).added.push(path);
	}
	
	pub fn removed(&mut self, uuid: Uuid, path: PathBuf) {
		self.get_node(uuid).removed.push(path);
	}
	
	pub fn debug_print(&self) {
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
