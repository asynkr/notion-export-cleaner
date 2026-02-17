use file_type::FileType;
use notion_object::NotionObject;

use clap::Parser;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use walkdir::WalkDir;

use crate::file_type::FileMapByName;
use crate::notion_object::ObjectsMapByName;

mod content_replacing;
mod constants;
mod file_type;
mod notion_object;
mod path_replacing;
mod uriencoding;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)] // Read from `Cargo.toml`
#[clap(rename_all = "kebab_case")]
struct NECArgs {
    /// The directory to walk
    #[arg(required = true)]
    input_dir: String,

    /// List of directory/file names to ignore, separated by commas.
    /// Typically, this can be used to ignore a `.git` directory.
    #[arg(short, long, value_name="FILE_OR_DIR", num_args(1..), value_delimiter = ',')]
    ignore: Vec<String>,

    /// Prevent contents or file name modification. The program will execute everything except the actual writing to file. Use this to test for errors.
    #[arg(short, long)]
    test: bool,
}

fn main() {
    let args = NECArgs::parse();

    let is_testing = args.test;
    let directory = args.input_dir;
    let directory_path = PathBuf::from(&directory);

    if is_testing {
        println!("RUNNING IN TEST MODE");
    }

    println!("Walking directory tree");
    let ignore: HashSet<&str> = HashSet::from_iter(args.ignore.iter().map(|s| s.as_str()));
    let file_map: FileMapByName = build_file_map(directory_path, ignore);
    let files_vec = file_map.values().flatten().collect::<Vec<&FileType>>();

    print_file_map_info(&file_map);

    println!("Building enriched objects from files");
    let objects = NotionObject::create_objects_from_file_map(&file_map);
    let mut objects_map: ObjectsMapByName = NotionObject::build_map_by_name(objects);
    
    path_replacing::resolve_new_names(&mut objects_map);

    let objects_vec = objects_map
        .values()
        .flatten()
        .collect::<Vec<&NotionObject>>();

    println!("Modifying contents of files");
    content_replacing::rename_refs_in_all_files(&files_vec, &objects_vec, is_testing);

    println!("Renaming files");
    path_replacing::rename_objects_files(&objects_vec, is_testing);

    println!("Renaming directories");
    path_replacing::rename_directories(&objects_vec, is_testing);
}

/// The file map is a map of file keys to a list of the entries matching this key.
/// See `FileType::get_file_key` for more information.
fn build_file_map(
    directory_path: PathBuf,
    ignore: HashSet<&str>,
) -> FileMapByName {
    let mut file_map: FileMapByName = HashMap::new();

    for entry in WalkDir::new(directory_path) {
        let entry = entry.unwrap(); // panic if error

        // Ignore hidden directories
        if entry
            .path()
            .components()
            .map(|c| c.as_os_str().to_str().unwrap())
            .collect::<HashSet<&str>>()
            .intersection(&ignore)
            .count()
            > 0
        {
            continue;
        }

        let ft = FileType::from(entry.path().to_path_buf());
        let file_key = ft.get_file_key().to_string();
        file_map.entry(file_key).or_default().push(ft);
    }

    file_map
}

fn print_file_map_info(file_map: &FileMapByName) {
    println!("Found:");

    let mut md_files = 0;
    let mut html_files = 0;
    let mut csv_files = 0;
    let mut csv_all_files = 0;
    let mut directories = 0;
    let mut other_txt_files = 0;
    let mut other_bin_files = 0;
    for ft_vec in file_map.values() {
        for ft in ft_vec.iter() {
            match ft {
                FileType::Markdown(_) => md_files += 1,
                FileType::Html(_) => html_files += 1,
                FileType::Csv(_) => csv_files += 1,
                FileType::CsvAll(_) => csv_all_files += 1,
                FileType::Dir(_) => directories += 1,
                FileType::OtherTxt(_) => other_txt_files += 1,
                FileType::OtherBin(_) => other_bin_files += 1,
            }
        }
    }
    println!("\t{} markdown files\n\t{} html files\n\t{} csv files\n\t{} csv_all files\n\t{} directories\n\t{} other text files\n\t{} other binary files",
        md_files, html_files, csv_files, csv_all_files, directories, other_txt_files, other_bin_files
    );
    println!("Total: {}", file_map.values().flatten().count());
}
