use std::{collections::HashMap, path::PathBuf};

use walkdir::WalkDir;

pub type DirMap = HashMap<String, PathBuf>;
pub type FileMap = HashMap<String, PathBuf>;
pub type CsvMap = HashMap<String, PathBuf>;
pub type NonMdFiles = Vec<PathBuf>;

pub fn build_maps<'a>(directory_path: PathBuf) -> (DirMap, FileMap, CsvMap, NonMdFiles) {
    let mut directories = DirMap::new();
    let mut files = FileMap::new();
    let mut csvs = CsvMap::new();
    let mut non_md_files = NonMdFiles::new();

    for entry in WalkDir::new(&directory_path) {
        let entry = entry.unwrap();
        let path = entry.path();
        let path_buf = path.to_path_buf();

        if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_str().unwrap();
            directories.insert(dir_name.to_string(), path_buf);
        } else {
            let name_uuid = path.file_stem().unwrap().to_str().unwrap();
            let extension = path.extension().unwrap().to_str().unwrap();

            match extension {
                "csv" => {
                    csvs.insert(name_uuid.to_string(), path_buf);
                }
                "md" => {
                    files.insert(name_uuid.to_string(), path_buf);
                }
                _ => {
                    non_md_files.push(path_buf);
                }
            }
        }
    }

    (directories, files, csvs, non_md_files)
}
