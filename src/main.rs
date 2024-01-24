use file_type::FileType;
use indicatif::ProgressIterator;
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    process::exit,
};
use walkdir::WalkDir;

use crate::objects::NotionObject;

mod file_type;
mod objects;

struct EntryRef {
    /// name + space + UUID (for uniqueness)
    name_uuid: String,
    /// name of the file without the uuid
    name: String,
    /// New name
    new_name: Option<String>,

    file_path: PathBuf,
    dir_path: Option<PathBuf>, // If the file has subpages, there will be a directory with the same name
    csv_path: Option<PathBuf>, // If the file is a database, there will be a csv file with the same name
    csv_all_path: Option<PathBuf>, // If the file is a database, there will also be a _all.csv file
}

impl EntryRef {
    fn get_name_uuid_from_path(path: &PathBuf) -> String {
        let file_name = path.file_stem().unwrap().to_str().unwrap();
        let name_uuid = file_name.to_string();
        name_uuid
    }

    fn from_file_path(
        file_path: PathBuf,
        directories: &HashMap<String, PathBuf>,
        csvs: &HashMap<String, PathBuf>,
    ) -> EntryRef {
        let name_uuid = EntryRef::get_name_uuid_from_path(&file_path);
        // name is the name without the uuid
        // uuid is the characters after the last space, but there might not be multiple spaces
        let last_space_index = name_uuid.rfind(' ').unwrap_or(0);
        let name = name_uuid[0..last_space_index].to_string();

        let dir_path = directories.get(&name_uuid).cloned();
        let csv_path = csvs.get(&name_uuid).cloned();
        let csv_all_path = csvs.get(&(name_uuid.clone() + "_all")).cloned();

        EntryRef {
            name_uuid,
            name,
            new_name: None,
            file_path,
            dir_path,
            csv_path,
            csv_all_path,
        }
    }
}

fn main() {
    let directory = env::args().nth(1).expect("no directory given");
    let directory_path = PathBuf::from(&directory);

    println!("Walking directory tree");
    let mut file_map: HashMap<String, Vec<FileType>> = HashMap::new();

    // To skip root
    for entry in WalkDir::new(&directory_path) {
        let ft = FileType::from(entry.unwrap().path().to_path_buf());
        let file_key = ft.get_file_key().to_string();
        file_map.entry(file_key).or_default().push(ft);
    }
    println!("Found:");
    {
        let mut md_files = 0;
        let mut csv_files = 0;
        let mut csv_all_files = 0;
        let mut directories = 0;
        let mut other_txt_files = 0;
        let mut other_bin_files = 0;
        for ft_vec in file_map.values() {
            for ft in ft_vec.iter() {
                match ft {
                    FileType::Markdown(_) => md_files += 1,
                    FileType::Csv(_) => csv_files += 1,
                    FileType::CsvAll(_) => csv_all_files += 1,
                    FileType::Dir(_) => directories += 1,
                    FileType::OtherTxt(_) => other_txt_files += 1,
                    FileType::OtherBin(_) => other_bin_files += 1,
                }
            }
        }
        println!("\t{} markdown files\n\t{} csv files\n\t{} csv_all files\n\t{} directories\n\t{} other text files\n\t{} other binary files",
            md_files, csv_files, csv_all_files, directories, other_txt_files, other_bin_files
        );
        println!("Total: {}", file_map.values().flatten().count());
    }

    println!("Building enriched objects from files");
    let objects = NotionObject::objects_from_map(&file_map);
    let mut objects_map = NotionObject::build_map_by_name(objects);
    NotionObject::find_new_names(&mut objects_map);
}
