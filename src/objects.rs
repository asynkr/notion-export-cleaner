use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use crate::file_type::FileType;

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
        obj: NotionObjectInfo,
        csv_all_path: Option<PathBuf>,
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
            let mut page_path = None;
            let mut csv_path = None;
            let mut csv_all_path = None;
            let mut dir_path = None;

            // Have we encountered a file type that is not a page, database, or directory?
            let mut non_standard_file_encountered = false;

            for file_type in file_types {
                match file_type {
                    FileType::Markdown(file_info) => {
                        page_path = Some(file_info.path.clone());
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

            if non_standard_file_encountered {
                assert!(page_path.is_none());
                assert!(csv_path.is_none());
                assert!(csv_all_path.is_none());
                assert!(dir_path.is_none());
                continue;
            }

            match (page_path, csv_path, csv_all_path, dir_path) {
                // Markdown file
                (Some(page_path), None, None, dir_path) => {
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
                (None, Some(csv_path), csv_all_path, dir_path) => {
                    let last_space_index = key
                        .rfind(' ')
                        .unwrap_or_else(|| panic!("No space in file name: {}", key));
                    let name = key[0..last_space_index].to_string();
                    let uuid = key[last_space_index + 1..].to_string();
                    notion_objects.push(NotionObject::Database {
                        obj: NotionObjectInfo {
                            path: csv_path,
                            name,
                            uuid,
                            dir_path,
                            new_name: None,
                        },
                        csv_all_path,
                    });
                }
                // Directory alone. we dont rename it, so skip it.
                (None, None, None, Some(_)) => {}
                // Invalid !
                (page_path, csv_path, csv_all_path, _) => panic!(
                    "Invalid paths (database files and page files have same key {}):\n{}\n{}\n{}",
                    key,
                    page_path.unwrap_or_default().display(),
                    csv_path.unwrap_or_default().display(),
                    csv_all_path.unwrap_or_default().display()
                ),
            }
        }

        notion_objects
    }
}

type ObjectsMapByName = HashMap<String, Vec<NotionObject>>;

impl NotionObject {
    fn get_name(&self) -> &str {
        match self {
            NotionObject::Page(info) => &info.name,
            NotionObject::Database { obj, .. } => &obj.name,
            NotionObject::OtherText { path } => path.file_stem().unwrap().to_str().unwrap(),
            NotionObject::OtherBinary { path } => path.file_stem().unwrap().to_str().unwrap(),
        }
    }

    /// Sets new_name for objects that can be renamed.
    fn try_set_new_name(&mut self, new_name: String) {
        match self {
            NotionObject::Page(info) => info.new_name = Some(new_name),
            NotionObject::Database { obj, .. } => obj.new_name = Some(new_name),
            // 'Other' files don't have to be renamed
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. } => {}
        }
    }

    fn get_path(&self) -> &PathBuf {
        match self {
            NotionObject::Page(info) => &info.path,
            NotionObject::Database { obj, .. } => &obj.path,
            NotionObject::OtherText { path } => path,
            NotionObject::OtherBinary { path } => path,
        }
    }

    fn is_other(&self) -> bool {
        match self {
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. } => true,
            _ => false,
        }
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
                    new_name_with_extension.push_str(".");
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
}
