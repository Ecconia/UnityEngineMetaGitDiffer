pub mod data {
	pub mod uuid;
	pub mod uuid_storage;
}

use crate::data::uuid::Uuid;
use crate::data::uuid_storage::UuidStorage;
use git2::{Delta, Diff, DiffDelta, DiffOptions, Repository};
use std::env;

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
				uuid_storage.added(uuid, path);
			}
			Delta::Added => {
				let hash = delta.new_file().id();
				let uuid = Uuid::from_blob_or_panic(&repo, hash);
				uuid_storage.added(uuid, path);
			}
			Delta::Deleted => {
				let hash = delta.old_file().id();
				let uuid = Uuid::from_blob_or_panic(&repo, hash);
				uuid_storage.removed(uuid, path);
			}
			Delta::Modified => {
				let uuid_from = Uuid::from_blob_or_panic(&repo, delta.old_file().id());
				let uuid_to = Uuid::from_blob_or_panic(&repo, delta.new_file().id());
				if uuid_from != uuid_to {
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
