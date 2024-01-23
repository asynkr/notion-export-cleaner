use std::{
    collections::{HashMap, HashSet}, env, fs, path::{Path, PathBuf}, process::exit
};
use indicatif::ProgressIterator;

mod maps;

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

    println!("Indexing files in {}", directory_path.display());
    let (directories, files, csvs, non_md_files) = maps::build_maps(directory_path);

    println!("Building objects from {} markdown files", files.len());
    let mut entries = HashMap::new();
    for file_path in files.values().progress() {
        let entry = EntryRef::from_file_path(file_path.clone(), &directories, &csvs);
        entries.insert(entry.name_uuid.clone(), entry);
    }

    println!("Managing duplicates");
    let mut seen: HashSet<String> = HashSet::new();
    let desired_names_name_uuids = entries.values().map(
        |entry| (entry.name.clone(), entry.name_uuid.clone())
    ).collect::<Vec<(String, String)>>();
    for (desired_name, name_uuid) in desired_names_name_uuids.iter().progress() {
        let mut name_that_works: String = desired_name.clone();

        let mut i = 1;
        while seen.contains(&name_that_works) {
            name_that_works = format!("{} {}", desired_name, i);
            i += 1;
        }

        seen.insert(name_that_works.clone());
        entries.get_mut(name_uuid).unwrap().new_name = Some(name_that_works);
    }
}
