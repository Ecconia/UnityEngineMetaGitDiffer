pub mod data {
	pub mod uuid;
	pub mod uuid_storage;
	pub mod path_tree_storage;
}

use crate::data::path_tree_storage::PathTreeStorage;
use crate::data::uuid::Uuid;
use crate::data::uuid_storage::UuidStorage;
use ecc_ansi_lib::ansi;
use git2::{Delta, Diff, DiffDelta, DiffOptions, Repository};
use std::env;
use std::path::Path;

fn main() {
	env::set_current_dir("../../SourceCode").unwrap();
	println!("Path: {}", env::current_dir().unwrap().display());
	
	let repo = match Repository::open(".") {
		Ok(repo) => repo,
		Err(e) => panic!("failed to open: {e}"),
	};
	let diff = create_diff_from_arguments(&repo);
	let diffs = gather_filtered_deltas_from_diff(&diff);
	println!("Unstaged: {}", diffs.len());
	
	let mut uuid_storage = UuidStorage::default();
	let mut addition_tree = PathTreeStorage::default();
	let mut removal_tree = PathTreeStorage::default();
	
	sort_deltas_into_storages(
		&repo, &diffs,
		&mut uuid_storage,
		&mut addition_tree, &mut removal_tree,
	);
	
	// uuid_storage.debug_print();
	// println!();
	
	// Currently just print the two trees. That is sufficient information for starters.
	// Eventually a bunch of optimizations and improvements to the printing should be added.
	println!(ansi!("«lr»By removal tree«»:"));
	removal_tree.debug_print(&uuid_storage, false);
	println!();
	
	println!(ansi!("«lg»By addition tree«»:"));
	addition_tree.debug_print(&uuid_storage, true);
}

fn sort_deltas_into_storages(
	repository: &Repository, diffs: &Vec<DiffDelta>,
	uuid_storage: &mut UuidStorage,
	addition_tree: &mut PathTreeStorage, removal_tree: &mut PathTreeStorage,
) {
	fn added(
		uuid_storage: &mut UuidStorage, addition_tree: &mut PathTreeStorage,
		path: &Path, uuid: Uuid
	) {
		if let Some(previous_entry) = uuid_storage.added(uuid, path.to_path_buf()) {
			println!(
				ansi!("«y»WARNING:«» Trying to add a file to Git with a Unity GUID ({}) that is already added to the Git via path '{}'\n"),
				uuid, previous_entry.display(),
			);
			println!(">> IGNORING newer path '{}'", path.display());
		} else {
			addition_tree.add_to_tree(path, uuid);
		}
	}
	
	fn removed(
		uuid_storage: &mut UuidStorage, removal_tree: &mut PathTreeStorage,
		path: &Path, uuid: Uuid
	) {
		if let Some(previous_entry) = uuid_storage.removed(uuid, path.to_path_buf()) {
			println!(
				ansi!("«y»WARNING:«» Trying to remove a file from Git with a Unity GUID ({}) that is already removed from the Git via path '{}'\n"),
				uuid, previous_entry.display(),
			);
			println!(">> IGNORING newer path '{}'", path.display());
		} else {
			removal_tree.add_to_tree(path, uuid);
		}
	}
	
	for delta in diffs.iter() {
		// When working with libgit2, it does not detect renames by default. Thus, only additions/removals & modifications.
		// This means that old/new paths should always be set and always be the same. If that is not the case something is wrong - stop then.
		if delta.new_file().path().is_none() || delta.old_file().path().is_none() || delta.new_file().path().unwrap() != delta.old_file().path().unwrap() {
			panic!("The path of the old/new file did not match or one/both had not been set: {:?} ||| {:?}", delta.old_file(), delta.new_file());
		}
		
		let path = delta.old_file().path().unwrap().to_path_buf();
		// Not sure why this would ever happen. But let's not take the chance.
		if path.iter().next().is_none() {
			panic!("Path for diff delta was empty. This should never happen.");
		}
		
		match delta.status() {
			Delta::Untracked => {
				// The work-directory file (at path) was not in Git and is freshly added.
				let uuid = Uuid::from_disk_or_panic(&path);
				added(uuid_storage, addition_tree, &path, uuid);
			}
			Delta::Added => {
				// The file (at path) is added to Git.
				let hash = delta.new_file().id();
				let uuid = Uuid::from_blob_or_panic(repository, hash);
				added(uuid_storage, addition_tree, &path, uuid);
			}
			Delta::Deleted => {
				// The file (at path) was removed from Git
				let hash = delta.old_file().id();
				let uuid = Uuid::from_blob_or_panic(repository, hash);
				removed(uuid_storage, removal_tree, &path, uuid);
			}
			Delta::Modified => {
				// The file path has not changed, but the content did.
				let uuid_from = Uuid::from_blob_or_panic(repository, delta.old_file().id());
				let uuid_to = Uuid::from_blob_or_panic(repository, delta.new_file().id());
				// For the purpose of this program, only care about this file, when the UUID changed.
				// As in all other cases, everything is expected and okay.
				if uuid_from != uuid_to {
					added(uuid_storage, addition_tree, &path, uuid_to);
					removed(uuid_storage, removal_tree, &path, uuid_from);
				}
			}
			_ => {
				panic!("Cannot yet handle diff delta type of {:?}", delta.status());
			}
		}
	}
}

fn gather_filtered_deltas_from_diff<'a>(diff: &'a Diff<'a>) -> Vec<DiffDelta<'a>> {
	diff.deltas().filter(|delta| {
		let old = delta.old_file().path();
		let new = delta.new_file().path();
		// New/Old paths are always the same (in my case).
		// Anyway, check if either path has the '.meta' file extension.
		old.is_some() && old.unwrap().to_str().unwrap().ends_with(".meta")
			|| new.is_some() && new.unwrap().to_str().unwrap().ends_with(".meta")
	}).collect()
}

fn create_diff_from_arguments(repo: &Repository) -> Diff {
	let mut diff_opts = DiffOptions::new();
	diff_opts.include_untracked(true);
	diff_opts.recurse_untracked_dirs(true);
	let head_commit = repo.head().unwrap().peel_to_commit().unwrap().tree().unwrap();
	repo.diff_tree_to_workdir_with_index(Some(&head_commit), Some(&mut diff_opts)).unwrap()
	
	// let commits = ("00e600757bc5984fde1dc5a1aea358150d5a4433", "9e83dcf63dab7ab17af8d85a87f8491e91407ede");
	// let a = repo.find_commit_by_prefix(commits.0).unwrap().tree().unwrap();
	// let b = repo.find_commit_by_prefix(commits.1).unwrap().tree().unwrap();
	// repo.diff_tree_to_tree(Some(&a), Some(&b), None).unwrap()
}
