use git2::{Diff, DiffOptions, Repository, Tree};
use std::{env, process};
use std::path::Path;
/*
	Supported argument format:
	./exe => Diff HEAD with workdir
	./exe <hash> => Diff commit with workdir
	./exe <hash> <hash> => Diff 2 commits
	Same again with <path to repository>. Any first <hash> argument that does not look like a hash is treated as <path>.
	./exe <path>
	./exe <path> <hash>
	./exe <path> <hash> <hash>
	Same again, but with explicit path (in case of collision with <hash>:
	./exe --path <path>
	./exe --path <path> <hash>
	./exe --path <path> <hash> <hash>
	Any other input will print the help:
	./exe anything-else => Help
 */

fn print_help_and_quit(error_message: &str) -> ! {
	eprintln!("{error_message}");
	eprintln!();
	eprintln!("Help: This tool will create a diff for a Unity Git repository and read the changed meta files to display which assets got added/removed/renamed.");
	eprintln!(" ./{} [[--path] <path>] [hash 1] [hash 2]", Path::new(&env::args().next().unwrap()).iter().next_back().unwrap().display());
	eprintln!(" - If <path> is provided, the current execution directory is changed. Should be the root folder of the repository.");
	eprintln!(" - If no <hash> is provided, the diff will be created between head commit and work directory.");
	eprintln!(" - If one <hash> is provided, the diff will be created between provided commit and work directory.");
	eprintln!(" - If two <hashes> are provided, the diff will be created between these two provided commits.");
	process::exit(1);
}

fn is_hash_like(input: &str) -> bool {
	input.len() <= 40 && input.bytes().map(|b| b as char).all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c) || ('A'..='F').contains(&c))
}

// It is kind of impossible (for me) to return Repository and Diff in the same method call.
// This is due to the fact that Diff borrows from Repository - which Rust seems to be highly allergic to.
// Thus parsing arguments is a two-stage operations. And some temporary data has to be passed over.
// TBI: Maybe solve this with an Arguments struct?
pub struct ArgumentTemporaryData {
	potential_hash_a: Option<String>,
	potential_hash_b: Option<String>,
}

pub fn parse_arguments_open_repo() -> (Repository, ArgumentTemporaryData) {
	let mut potential_path = None;
	let mut potential_hash_a : Option<String> = None;
	let mut potential_hash_b : Option<String> = None;
	
	// Ensure there are at most 4 arguments:
	// (./exe) --path <path> <hash> <hash>
	if env::args().len() > (1 + 4) {
		print_help_and_quit("Too many arguments.");
	}
	let mut argument_iterator = env::args().skip(1).peekable(); // Skip executable path.
	
	// Only triggers when the first argument is '--path'
	// Consumes 2 arguments if triggers.
	if let Some(path_hint) = argument_iterator.peek() {
		if path_hint.eq_ignore_ascii_case("--path") {
			argument_iterator.next();
			if let Some(path) = argument_iterator.next() {
				potential_path = Some(path);
			} else {
				print_help_and_quit("Missing path argument after '--path'.");
			}
		}
	}
	
	// Assume no path and consume up to two hash arguments
	if let Some(hint_1) = argument_iterator.next() {
		potential_hash_a = Some(hint_1);
	}
	if let Some(hint_2) = argument_iterator.next() {
		potential_hash_b = Some(hint_2);
	}
	
	// If there still is an argument now, there was no '--path <path>' previously.
	// Thus, the arguments are '<path> <hash> <hash>'. Shift them appropriately and consume the last hash.
	if let Some(hint_3) = argument_iterator.next() {
		potential_path = potential_hash_a;
		potential_hash_a = potential_hash_b;
		potential_hash_b = Some(hint_3);
	}
	
	// At this point it is possible, that just the path and an optional hash was provided:
	// <path>
	// <path> <hash>
	if potential_path.is_none() && potential_hash_a.is_some() {
		// Check if the first argument could not be a hash:
		if !is_hash_like(potential_hash_a.as_ref().unwrap()) {
			// Cannot be a hash, shift it to be a path.
			potential_path = potential_hash_a;
			potential_hash_a = potential_hash_b;
			potential_hash_b = None; // If it was not already.
		}
	}
	// Find Git repository:
	if let Some(argument_path) = potential_path {
		let path = Path::new(&argument_path);
		if !path.exists() || !path.is_dir(){
			print_help_and_quit(&format!("No existing folder at '{argument_path}'"));
		}
		env::set_current_dir(path).expect("Failed to change directory.");
	}
	let repo = match Repository::open(".") {
		Ok(repo) => repo,
		Err(e) => {
			eprintln!("Did not find OR could not open repository at location: {}", env::current_dir().unwrap().display());
			eprintln!(" Details (by gitlib2): {e}");
			process::exit(1);
		},
	};
	println!("Using Git repository at path: {}", env::current_dir().unwrap().display());
	
	(repo, ArgumentTemporaryData {
		potential_hash_a,
		potential_hash_b,
	})
}

pub fn parse_arguments_create_diff(repo: &Repository, temp_data: ArgumentTemporaryData) -> Diff {
	// Validate arguments:
	fn validate_hash<'a>(repo: &'a Repository, hash_text: &str) -> Tree<'a> {
		if !is_hash_like(hash_text) {
			print_help_and_quit(&format!("Argument does not appear to be a git commit hash: '{hash_text}'"));
		}
		match repo.find_commit_by_prefix(hash_text) {
			Ok(commit) => match commit.tree() {
				Ok(tree) => tree,
				Err(error) => print_help_and_quit(&format!("Did not find OR could not load commit hash: {hash_text}\nDetails (by gitlib2): {error}"))
			}
			Err(error) => print_help_and_quit(&format!("Did not find OR could not load commit hash: {hash_text}\nDetails (by gitlib2): {error}"))
		}
	}
	let hash_first = temp_data.potential_hash_a.map(|arg| validate_hash(repo, &arg));
	let hash_second = temp_data.potential_hash_b.map(|arg| validate_hash(repo, &arg));
	
	if let Some(hash_second) = hash_second {
		let hash_first = hash_first.unwrap();
		repo.diff_tree_to_tree(Some(&hash_first), Some(&hash_second), None).unwrap()
	} else {
		let first = if let Some(hash_first) = hash_first {
			hash_first
		} else {
			repo.head().unwrap().peel_to_commit().unwrap().tree().unwrap()
		};
		
		let mut diff_opts = DiffOptions::new();
		diff_opts.include_untracked(true);
		diff_opts.recurse_untracked_dirs(true);
		repo.diff_tree_to_workdir_with_index(Some(&first), Some(&mut diff_opts)).unwrap()
	}
}
