use crate::data::uuid::Uuid;
use crate::data::uuid_storage::UuidStorage;
use ecc_ansi_lib::ansi;
use std::cmp::min;
use std::collections::HashMap;
use std::path::Path;

#[derive(Default)]
struct TreeNode {
	uuid: Option<Uuid>,
	entries: HashMap<String, TreeNode>,
}

#[derive(Default)]
pub struct PathTreeStorage {
	root_entries: HashMap<String, TreeNode>,
}

impl PathTreeStorage {
	pub fn add_to_tree(&mut self, path: &Path, uuid: Uuid) {
		// Remove the extension (".meta") from the path:
		let mut path = path.to_path_buf();
		path.set_extension("");
		
		let mut path_iterator = path.iter();
		
		// Resolve the very first node. This is an explicit step as root cannot have a UUID.
		let first_element = path_iterator.next().unwrap(); // Caller did ensure that the path is not empty.
		let mut current_node = self.root_entries.entry(first_element.to_str().unwrap().to_owned()).or_default();
		
		// Resolve all other nodes for this path. The current_node will then point towards the folder/file which gets a UUID.
		for element in path_iterator {
			current_node = current_node.entries.entry(element.to_str().unwrap().to_owned()).or_default();
		}
		
		// Finally set the UUID. But confirm, that there is not already a UUID for this path.
		// On the other side, the UUID Storage already checks for this issue - thus this should never trigger.
		if let Some(previous_entry) = current_node.uuid {
			// TODO: Find a better way to gracefully handle this case. For now assume that developers used their Git responsibly and did not mess up...
			panic!("For path '{}' two UUIDs got added or removed ({previous_entry} & {uuid})- normally a gUid is supposed to be UNIQUE (to a single path).", path.display())
		}
		current_node.uuid = Some(uuid);
	}
	
	pub fn debug_print(&self, uuid_storage: &UuidStorage, is_adding: bool) {
		fn add_flipped<'a>(stack: &mut Vec<(String, &'a TreeNode, String, String)>, map: &'a HashMap<String, TreeNode>, prefix: String){
			let mut list : Vec<_> = map.iter().collect();
			// HashMaps are ordered with a random seed - sort to ensure consistent output order.
			list.sort_by_key(|(path, _)| *path);
			
			// Collect all folders. Given that the folders had been sorted before (gitlib2 ordering), they are reversely added.
			// This ensures that the first one gets added on the stack last - so that it gets popped first.
			stack.extend(list.into_iter().enumerate().rev().map(|(index, (path, node))| (
				path.to_owned(),
				node,
				// Tree-building magic. There is a main-prefix for the first child-node line.
				// And a sub-prefix to prefix all lines of grand-child-nodes.
				if index == map.len() - 1 { format!("{prefix}└─") } else { format!("{prefix}├─") },
				if index == map.len() - 1 { format!("{prefix}  ") } else { format!("{prefix}│ ") },
			)));
		}
		
		let mut stack = Vec::new();
		// Add root level entries:
		add_flipped(&mut stack, &self.root_entries, "".to_owned());
		
		while let Some((path_element, node, prefix_main, prefix_sub)) = stack.pop() {
			// Construct a suffix fitting details to this folder entry:
			let suffix = if let Some(uuid) = node.uuid {
				let storage_entry = uuid_storage.lookup.get(&uuid).unwrap();
				// SAFETY: The following code gets added/removed reference - if it is set it also takes the other reference.
				// This is not an issue - by code design:
				// When going over the addition tree paths - we know when a UUID exists there must exist a UUID-Addition path entry in the UUID-Storage.
				// Thus, one only has to check if a removal exists - an addition always exists. The same applies for the removal tree.
				if is_adding {
					let optional_primary_path = &storage_entry.removed;
					if let Some(primary_path) = optional_primary_path {
						let secondary_path = storage_entry.added.as_ref().unwrap(); // See safety comment.
						&format!(" <= '{}'", Self::highlight_path_change(primary_path, secondary_path))
					} else {
						&format!(ansi!(" «lg»ADDED«» {}"), uuid)
					}
				} else {
					let optional_primary_path = &storage_entry.added;
					if let Some(primary_path) = optional_primary_path {
						let secondary_path = storage_entry.removed.as_ref().unwrap(); // See safety comment.
						&format!(" => '{}'", Self::highlight_path_change(primary_path, secondary_path))
					} else {
						&format!(ansi!(" «lr»REMOVED«» {}"), uuid)
					}
				}
			} else {
				// No UUID for this folder, thus no means to add details.
				""
			};
			println!(ansi!("{}«w»{}«»:{}"), prefix_main, path_element, suffix);
			// Add child folders for this folder:
			add_flipped(&mut stack, &node.entries, prefix_sub);
		}
	}
	
	fn highlight_path_change(main_path: &Path, reference_path: &Path) -> String {
		// Get the length of the smaller path, to later when looping over paths never run out-of-bounds.
		let min_part_count = min(
			main_path.iter().count(),
			reference_path.iter().count(),
		);
		// The idea is to find detect matching start/end paths.
		
		// Count matching starting & ending path elements:
		fn count_same_parts<T: Iterator>(max_iteration: usize, mut iterator_a: T, mut iterator_b: T) -> usize
			where <T as Iterator>::Item: PartialEq {
			let mut count = 0;
			for _ in  0..max_iteration {
				if iterator_a.next().unwrap() == iterator_b.next().unwrap() {
					count += 1;
				} else {
					break;
				}
			}
			count
		}
		let start_index = count_same_parts(min_part_count, &mut main_path.iter(), &mut reference_path.iter());
		let end_index = count_same_parts(min_part_count, &mut main_path.iter().rev(), &mut reference_path.iter().rev());
		
		let center_parts = main_path.iter().count() as isize - start_index as isize - end_index as isize;
		if center_parts < 0 {
			unreachable!("Apparently something is wrong with the code to highlight path differences. Managed to ");
		}
		
		let mut output = String::new();
		let mut main_iter = main_path.iter();
		
		// Print the prefix path parts:
		for _ in 0..start_index {
			output.push_str(ansi!("«gr»"));
			output.push_str(main_iter.next().unwrap().to_str().unwrap());
			output.push_str(ansi!("«w»/"));
		}
		// Print the non-matching center parts in blue:
		for _ in 0..center_parts {
			output.push_str(ansi!("«lb»"));
			output.push_str(main_iter.next().unwrap().to_str().unwrap());
			output.push_str(ansi!("«w»/"));
		}
		// If there is no center part, highlight the separating / between pre/suffix:
		if center_parts == 0 && !output.is_empty() {
			output.pop().unwrap();
			output.push_str(ansi!("«lb»/"));
		}
		// Print the suffix path parts:
		for _ in 0..end_index {
			output.push_str(ansi!("«gr»"));
			output.push_str(main_iter.next().unwrap().to_str().unwrap());
			output.push_str(ansi!("«w»/"));
		}
		// Remove the trailing / from the path:
		output.pop().unwrap();
		output.push_str(ansi!("«»"));
		
		output
	}
}
