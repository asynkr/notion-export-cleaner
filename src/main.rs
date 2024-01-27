use file_type::FileType;

use clap::Parser;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use walkdir::WalkDir;

use crate::objects::NotionObject;

mod file_type;
mod objects;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)] // Read from `Cargo.toml`
#[clap(rename_all = "kebab_case")]
struct NECArgs {
    /// The directory to walk
    #[arg(required = true)]
    input_dir: String,

    /// List of directory/file names to ignore.
    /// Typically, this can be used to ignore a `.git` directory.
    #[arg(short, long, value_name="FILE_OR_DIR", num_args(0..))]
    ignore: Vec<String>,
}

fn main() {
    let args = NECArgs::parse();


    let directory = args.input_dir;
    let directory_path = PathBuf::from(&directory);

    println!("Walking directory tree");
    let mut file_map: HashMap<String, Vec<FileType>> = HashMap::new();

    let ignore: HashSet<&str> = HashSet::from_iter(args.ignore.iter().map(|s| s.as_str()));

    for entry in WalkDir::new(directory_path) {
        let entry = entry.unwrap();
        if entry
            .path()
            .components()
            .map(|c| c.as_os_str().to_str().unwrap())
            .collect::<HashSet<&str>>() // Ignore hidden directories
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
    println!("Found:");
    {
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

    println!("Building enriched objects from files");
    let objects = NotionObject::objects_from_map(&file_map);
    let mut objects_map = NotionObject::build_map_by_name(objects);
    NotionObject::find_new_names(&mut objects_map);

    println!("Modifying contents of files");
    NotionObject::rename_refs_in_all_files(
        file_map.values().flatten().collect::<Vec<&FileType>>(),
        objects_map
            .values()
            .flatten()
            .collect::<Vec<&NotionObject>>(),
    );

    println!("Renaming files");
    NotionObject::rename_objects_files(
        objects_map
            .values()
            .flatten()
            .collect::<Vec<&NotionObject>>(),
    );

    println!("Renaming directories");
    NotionObject::rename_directories(
        objects_map
            .values()
            .flatten()
            .collect::<Vec<&NotionObject>>(),
    );
}
