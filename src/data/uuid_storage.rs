use crate::data::uuid::Uuid;
use ecc_ansi_lib::ansi;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Default)]
#[derive(Clone)]
pub struct UuidStorageEntry {
	pub added: Option<PathBuf>,
	pub removed: Option<PathBuf>,
}

#[derive(Default)]
pub struct UuidStorage {
	pub lookup: HashMap<Uuid, UuidStorageEntry>
}

impl UuidStorage {
	fn get_or_create_node(&mut self, uuid: Uuid) -> &mut UuidStorageEntry {
		self.lookup.entry(uuid).or_default()
	}
	
	pub fn added(&mut self, uuid: Uuid, path: PathBuf) -> Option<&PathBuf> {
		Self::set(&mut self.get_or_create_node(uuid).added, path)
	}
	
	pub fn removed(&mut self, uuid: Uuid, path: PathBuf) -> Option<&PathBuf> {
		Self::set(&mut self.get_or_create_node(uuid).removed, path)
	}
	
	fn set(option: &mut Option<PathBuf>, mut path: PathBuf) -> Option<&PathBuf> {
		path.set_extension("");
		if option.is_none() {
			*option = Some(path);
			None
		} else {
			option.as_ref()
		}
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
