use indicatif::ProgressIterator;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

use crate::file_type::{FileInfo, FileType};

/// The `index.html` file has a `index` key.
/// It shouldn't be totally ignored because it has content to be modified,
/// but it shouldn't be renamed. So it shouldn't be a Page object.
const INDEX_KEY: &str = "index";

pub struct NotionObjectInfo {
    /// The path to the file
    path: PathBuf,
    /// The name of the file without the UUID
    name: String,
    /// The UUID of the file
    uuid: String,

    /// The path to the directory with the same name as the file, if it exists
    dir_path: Option<PathBuf>,

    /// The new name of the file.
    /// Best case: its `name`.
    /// Worst case: its `name` + space + a number.
    new_name: Option<String>,
}

pub enum NotionObject {
    Page(NotionObjectInfo),
    Database {
        obj_info: NotionObjectInfo,
        csv_all_path: Option<PathBuf>,
        html_path: Option<PathBuf>,
    },
    OtherText {
        path: PathBuf,
    },
    OtherBinary {
        path: PathBuf,
    },
}

impl NotionObject {
    pub fn objects_from_map(all_files: &HashMap<String, Vec<FileType>>) -> Vec<NotionObject> {
        let mut notion_objects = Vec::new();

        for (key, file_types) in all_files.iter() {
            let mut md_path = None;
            let mut html_path = None;
            let mut csv_path = None;
            let mut csv_all_path = None;
            let mut dir_path = None;

            // Have we encountered a file type that is not a page, database, or directory?
            let mut non_standard_file_encountered = false;

            for file_type in file_types {
                match file_type {
                    FileType::Markdown(file_info) => {
                        md_path = Some(file_info.path.clone());
                    }
                    FileType::Html(file_info) => {
                        html_path = Some(file_info.path.clone());
                    }
                    FileType::Csv(file_info) => {
                        csv_path = Some(file_info.path.clone());
                    }
                    FileType::CsvAll(file_info) => {
                        csv_all_path = Some(file_info.path.clone());
                    }
                    FileType::OtherTxt(path) => {
                        notion_objects.push(NotionObject::OtherText { path: path.clone() });
                        non_standard_file_encountered = true;
                    }
                    FileType::OtherBin(path) => {
                        notion_objects.push(NotionObject::OtherBinary { path: path.clone() });
                        non_standard_file_encountered = true;
                    }
                    FileType::Dir(file_info) => {
                        dir_path = Some(file_info.path.clone());
                    }
                }
            }

            if key == INDEX_KEY {
                assert!(file_types.len() == 1);
                assert!(html_path.is_some());
                notion_objects.push(NotionObject::OtherText {
                    path: html_path.unwrap(),
                });
                continue;
            }

            if non_standard_file_encountered {
                assert!(md_path.is_none());
                assert!(html_path.is_none());
                assert!(csv_path.is_none());
                assert!(csv_all_path.is_none());
                assert!(dir_path.is_none());
                continue;
            }

            match (md_path, html_path, csv_path, csv_all_path, dir_path) {
                // Page:
                // md file
                (Some(page_path), None, None, None, dir_path)
                // or html file
                | (None, Some(page_path), None, None, dir_path) => {
                    let last_space_index = key
                        .rfind(' ')
                        .unwrap_or_else(|| panic!("No space in file name: {}", key));
                    let name = key[0..last_space_index].to_string();
                    let uuid = key[last_space_index + 1..].to_string();
                    notion_objects.push(NotionObject::Page(NotionObjectInfo {
                        path: page_path,
                        name,
                        uuid,
                        dir_path,
                        new_name: None,
                    }));
                }
                // Database file
                // A database file can have an associated html file
                (None, html_file, Some(csv_path), csv_all_path, dir_path) => {
                    let last_space_index = key
                        .rfind(' ')
                        .unwrap_or_else(|| panic!("No space in file name: {}", key));
                    let name = key[0..last_space_index].to_string();
                    let uuid = key[last_space_index + 1..].to_string();
                    notion_objects.push(NotionObject::Database {
                        obj_info: NotionObjectInfo {
                            path: csv_path,
                            name,
                            uuid,
                            dir_path,
                            new_name: None,
                        },
                        csv_all_path,
                        html_path: html_file,
                    });
                }
                // Directory alone. we dont rename it, so skip it.
                // (All renamable directories are associated with a page or a database)
                (None, None, None, None, Some(_)) => {}
                // Invalid !
                (md_path, html_path, csv_path, csv_all_path, _) => panic!(
                    "Invalid paths (database files and page files have same key {}):\n{}\n{}\n{}\n{}",
                    key,
                    md_path.unwrap_or_default().display(),
                    html_path.unwrap_or_default().display(),
                    csv_path.unwrap_or_default().display(),
                    csv_all_path.unwrap_or_default().display()
                ),
            }
        }

        notion_objects
    }

    fn get_name_uuid(&self) -> String {
        match self {
            NotionObject::Page(obj_info) | NotionObject::Database { obj_info, .. } => {
                format!("{} {}", obj_info.name, obj_info.uuid)
            }
            NotionObject::OtherText { path } | NotionObject::OtherBinary { path } => {
                path.file_stem().unwrap().to_str().unwrap().to_string()
            }
        }
    }

    fn is_renamable(&self) -> bool {
        match self {
            NotionObject::Page(_) | NotionObject::Database { .. } => true,
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. } => false,
        }
    }

    fn has_dir(&self) -> bool {
        match self {
            NotionObject::Page(info) | NotionObject::Database { obj_info: info, .. } => {
                info.dir_path.is_some()
            }
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. } => false,
        }
    }

    fn get_dir(&self) -> Option<&PathBuf> {
        match self {
            NotionObject::Page(info) | NotionObject::Database { obj_info: info, .. } => {
                info.dir_path.as_ref()
            }
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. } => None,
        }
    }
}

type ObjectsMapByName = HashMap<String, Vec<NotionObject>>;

impl NotionObject {
    fn get_name(&self) -> &str {
        match self {
            NotionObject::Page(info) => &info.name,
            NotionObject::Database { obj_info: obj, .. } => &obj.name,
            NotionObject::OtherText { path } => path.file_stem().unwrap().to_str().unwrap(),
            NotionObject::OtherBinary { path } => path.file_stem().unwrap().to_str().unwrap(),
        }
    }

    /// Sets new_name for objects that can be renamed.
    fn try_set_new_name(&mut self, new_name: String) {
        match self {
            NotionObject::Page(info) => info.new_name = Some(new_name),
            NotionObject::Database { obj_info: obj, .. } => obj.new_name = Some(new_name),
            // 'Other' files don't have to be renamed
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. } => {}
        }
    }

    fn get_path(&self) -> &PathBuf {
        match self {
            NotionObject::Page(info) => &info.path,
            NotionObject::Database { obj_info: obj, .. } => &obj.path,
            NotionObject::OtherText { path } => path,
            NotionObject::OtherBinary { path } => path,
        }
    }

    fn is_other(&self) -> bool {
        matches!(
            self,
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. }
        )
    }

    pub fn build_map_by_name(notion_objects: Vec<NotionObject>) -> ObjectsMapByName {
        let mut map: ObjectsMapByName = HashMap::new();
        for notion_object in notion_objects {
            let name = notion_object.get_name().to_string();
            map.entry(name).or_default().push(notion_object);
        }
        map
    }

    /// Find a new name for this object.
    /// ASSUMPTION: No directory can exist without a page or a database.
    /// This assumption has been checked in `objects_from_map`, which makes sure either a page or a database exists for each entry.
    pub fn find_new_names(all_objects_by_name: &mut ObjectsMapByName) {
        for (name, objects) in all_objects_by_name {
            if objects.len() == 1 {
                objects[0].try_set_new_name(name.clone());
                continue;
            }

            // paths that we expect after renaming the files (not the directories)
            let expected_paths = objects
                .iter()
                .map(|obj| {
                    if obj.is_other() {
                        return None;
                    }
                    let path = obj.get_path();
                    let extension = path.extension().unwrap().to_str().unwrap();

                    let mut new_name_with_extension = name.clone();
                    new_name_with_extension.push('.');
                    new_name_with_extension.push_str(extension);

                    Some(path.with_file_name(new_name_with_extension))
                })
                .collect::<Vec<_>>();

            let mut new_paths_seen = HashSet::new();
            for i in 0..objects.len() {
                let obj = &mut objects[i];
                match expected_paths[i].as_ref() {
                    // This object is not a page or a database, so we don't need to rename it
                    None => continue,
                    Some(expected_path) => {
                        let mut expected_path = expected_path.clone();

                        let mut add = 1;
                        // This exact path already exists, so we need to add a number to the end of the name
                        while new_paths_seen.contains(&expected_path) {
                            expected_path = expected_path
                                .with_file_name(
                                    format!("{} {}", name, add), // No extension at this point, with_file_name removes it
                                )
                                .with_extension(expected_path.extension().unwrap_or_default());
                            add += 1;
                        }
                        new_paths_seen.insert(expected_path.clone());

                        obj.try_set_new_name(
                            expected_path
                                .file_stem()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string(),
                        );
                    }
                }
            }
        }
    }

    fn rename_refs_in_file(&self, file_contents: &str) -> String {
        let mut new_contents: String = file_contents.to_string();

        let old_name = self.get_name_uuid();
        let new_name = match self {
            NotionObject::Page(obj_info) | NotionObject::Database { obj_info, .. } => {
                obj_info.new_name.as_ref().unwrap().as_str()
            }
            _ => panic!("non-page, non-database object wont be renamed"),
        };

        let old_name_encoded = urlencoding::encode(&old_name).into_owned();
        let new_name_encoded = urlencoding::encode(new_name).into_owned();

        // Renames all references
        // files, csv_all, directories, etc
        new_contents = new_contents.replace(&old_name, new_name);
        new_contents = new_contents.replace(&old_name_encoded, &new_name_encoded);

        new_contents
    }

    pub fn rename_refs_in_all_files(all_files: Vec<&FileType>, all_objects: Vec<&NotionObject>) {
        for file in all_files.iter().progress() {
            let path = match file {
                FileType::Markdown(FileInfo { path, .. })
                | FileType::Html(FileInfo { path, .. })
                | FileType::Csv(FileInfo { path, .. })
                | FileType::CsvAll(FileInfo { path, .. })
                | FileType::OtherTxt(path) => path,
                _ => continue,
            };

            let contents = fs::read_to_string(path).unwrap(); // Should not panic, file should be readable
            let mut new_contents = contents.clone();

            for object in all_objects.iter().filter(|obj| obj.is_renamable()) {
                new_contents = object.rename_refs_in_file(&new_contents);
            }

            if contents != new_contents {
                fs::write(path, new_contents).unwrap(); // Should not panic, file should be writable
            }
        }
    }

    /// Rename the file and its associated csv_all if any.
    fn rename_object_files(&self) {
        let old_path = self.get_path();
        match self {
            NotionObject::Page(obj_info) | NotionObject::Database { obj_info, .. } => {
                let new_path = old_path
                    .with_file_name(obj_info.new_name.as_ref().unwrap())
                    .with_extension(old_path.extension().unwrap());
                fs::rename(old_path, new_path).unwrap(); // Should not panic
            }
            _ => panic!("non-page, non-database object wont be renamed"),
        }

        if let NotionObject::Database {
            obj_info,
            csv_all_path: Some(old_csv_all_path),
            ..
        } = self
        {
            // Rename also the csv_all
            let new_csv_all_path = old_csv_all_path
                .with_file_name(obj_info.new_name.as_ref().unwrap().to_owned() + "_all")
                .with_extension(old_csv_all_path.extension().unwrap());

            fs::rename(old_csv_all_path, new_csv_all_path).unwrap(); // Should not panic
        }

        if let NotionObject::Database {
            obj_info,
            html_path: Some(old_html_path),
            ..
        } = self
        {
            // Rename also the html
            let new_html_path = old_html_path
                .with_file_name(obj_info.new_name.as_ref().unwrap())
                .with_extension(old_html_path.extension().unwrap());

            fs::rename(old_html_path, new_html_path).unwrap(); // Should not panic
        }
    }

    pub fn rename_objects_files(all_objects: Vec<&NotionObject>) {
        for object in all_objects
            .iter()
            .progress()
            .filter(|obj| obj.is_renamable())
        {
            object.rename_object_files();
        }
    }

    fn rename_dir(&self) {
        let old_dir_path = self.get_dir().unwrap();
        match self {
            NotionObject::Page(obj_info) | NotionObject::Database { obj_info, .. } => {
                let new_dir_path = old_dir_path.with_file_name(obj_info.new_name.as_ref().unwrap());
                fs::rename(old_dir_path, new_dir_path).unwrap(); // Should not panic
            }
            _ => panic!("non-page, non-database object dont have a directory"),
        }
    }

    pub fn rename_directories(all_objects: Vec<&NotionObject>) {
        let mut all_objects_sorted_by_dir_path_len_highest_first = all_objects
            .iter()
            .filter(|obj| obj.has_dir())
            .copied()
            .collect::<Vec<&NotionObject>>();
        all_objects_sorted_by_dir_path_len_highest_first
            .sort_by_key(|obj| obj.get_dir().unwrap().components().count());
        all_objects_sorted_by_dir_path_len_highest_first.reverse();

        // Rename dirs by deepest first, to avoid any conflicts
        // (Renaming a child dir after its parent has been renamed would cause a panic)
        for object in all_objects_sorted_by_dir_path_len_highest_first
            .iter()
            .progress()
        {
            object.rename_dir();
        }
    }
}
