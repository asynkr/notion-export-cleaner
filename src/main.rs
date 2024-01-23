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

    println!("Replacing files contents");
    let all_paths: Vec<&PathBuf> = files.values().chain(csvs.values()).chain(non_md_files.iter()).collect();
    for path in all_paths.iter().progress() {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            _ => { continue; }
        };
        let mut new_contents = contents.clone();

        for entry in entries.values() {
            let old_name = &entry.name_uuid;
            let old_name_encoded = urlencoding::encode(&entry.name_uuid).into_owned();

            let new_name =  entry.new_name.as_ref().unwrap();
            let new_name_encoded = urlencoding::encode(&new_name).into_owned();

            new_contents = new_contents.replace(old_name, new_name);
            new_contents = new_contents.replace(&old_name_encoded, &new_name_encoded);
        }

        if contents != new_contents {
            fs::write(path, new_contents).unwrap();
        }
    }

    println!("Renaming files");
    for entry in entries.values().progress() {
        let old_path = &entry.file_path;
        let new_path = old_path
            .with_file_name(&entry.new_name.as_ref().unwrap())
            .with_extension(old_path.extension().unwrap());
        let result = fs::rename(old_path, new_path);
        if let Err(e) = result {
            println!("Error renaming file: {}", e);
        }
    }

    println!("Renaming csvs");
    for entry in entries.values().progress() {
        let old_path = &entry.csv_path;
        let new_path = old_path
            .as_ref()
            .map(|path| path.with_file_name(&entry.new_name.as_ref().unwrap()))
            .map(|path| path.with_extension("csv"));
        if let Some(new_path) = new_path {
            let result = fs::rename(old_path.as_ref().unwrap(), new_path);
            if let Err(e) = result {
                println!("Error renaming file: {}", e);
            }
        }
    }

    println!("Renaming directories");
    // Sort entries by dir_path length inverted, so that we rename the subdirectories first
    let mut entries_sorted_by_dir_path = entries.values().collect::<Vec<&EntryRef>>();
    entries_sorted_by_dir_path.sort_by_key(|entry| -(entry.dir_path.clone().unwrap_or_default().to_str().unwrap().len() as isize));
    
    for entry in entries_sorted_by_dir_path.iter().progress() {
        let old_path = &entry.dir_path;
        let new_path = old_path
            .as_ref()
            .map(|path| path.with_file_name(&entry.new_name.as_ref().unwrap()));
        if let Some(new_path) = new_path {
            let result = fs::rename(old_path.as_ref().unwrap(), new_path);
            if let Err(e) = result {
                println!("Error renaming file: {}", e);
            }
        }
    }
}
