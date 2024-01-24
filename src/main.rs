use file_type::FileType;

use std::{collections::HashMap, env, path::PathBuf};
use walkdir::WalkDir;

use crate::objects::NotionObject;

mod file_type;
mod objects;

fn main() {
    let directory = env::args().nth(1).expect("no directory given");
    let directory_path = PathBuf::from(&directory);

    println!("Walking directory tree");
    let mut file_map: HashMap<String, Vec<FileType>> = HashMap::new();

    // To skip root
    for entry in WalkDir::new(directory_path) {
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
