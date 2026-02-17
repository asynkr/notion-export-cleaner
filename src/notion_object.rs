use std::{collections::HashMap, path::PathBuf};

use crate::{file_type::{FileMapByName, FileType}, uriencoding};

/// The `index.html` file has a `index` key.
/// It shouldn't be totally ignored because it has content to be modified,
/// but it shouldn't be renamed. So it shouldn't be a Page object.
const INDEX_KEY: &str = "index";

pub type ObjectsMapByName = HashMap<String, Vec<NotionObject>>;

/// Stores the info of a notion object.
/// These info are common for pages and databases.
pub struct NotionObjectInfo {
    /// The path to the file
    pub path: PathBuf,
    /// The name of the file without the UUID
    pub name: String,
    /// The UUID of the file
    pub uuid: String,
    /// Name + space + UUID
    pub old_name: String,

    /// The path to the directory with the same name as the file, if it exists
    pub dir_path: Option<PathBuf>,

    /// The new name of the file.
    /// Best case (this file is the only one to want this name): its `name`.
    /// Worst case (multiple candidates): its `name` + space + a number.
    pub new_name: Option<String>,
}

impl NotionObjectInfo {
    pub fn new(path: PathBuf, name: String, uuid: String, dir_path: Option<PathBuf>) -> Self {
        let old_name = format!("{} {}", &name, &uuid);
        Self{ path, name, uuid, old_name, dir_path, new_name: None }
    }
}

/// Stores the specific info of a database object.
pub struct NotionDatabaseInfo {
    /// A database can have both "name uuid.csv" and "name uuid_all.csv".
    /// The "name uuid.csv" file is considered the "main" file as far as NotionObject is concerned,
    /// but in reality it's a view of the database, whose entries are stored unfiltered in the "name uuid_all.csv" file.
    csv_all_path: Option<PathBuf>,
    /// When exporting as html,
    /// the database will have an html file to display the view.
    /// This is the field that stores the path to that file.
    html_path: Option<PathBuf>,
}

pub struct ReplacableReferences {
    pub old_ref: String,
    pub new_ref: String
}

pub struct ReplacableReferencesContainer {
    pub global_references: [ReplacableReferences; 4],
}

impl ReplacableReferencesContainer {
    pub fn build_from_name(old_name: &str, new_name: &str) -> Self {
        
        // Some of the references are uri-encoded
        // Some of the references are html-encoded
        // Others, a mix of both
        
        let old_name_html_encoded = html_escape::encode_safe(&old_name);
        let new_name_html_encoded = html_escape::encode_safe(&new_name);

        Self { global_references: [
                    ReplacableReferences{ old_ref: uriencoding::encode(&old_name).into_owned(), new_ref: uriencoding::encode(&new_name).into_owned() },
                    ReplacableReferences{ old_ref: html_escape::encode_safe(&old_name).into_owned(), new_ref: html_escape::encode_safe(&new_name).into_owned() },
                    ReplacableReferences{ old_ref: uriencoding::encode(&old_name_html_encoded).into_owned(), new_ref: uriencoding::encode(&new_name_html_encoded).into_owned() },
                    ReplacableReferences{ old_ref: old_name.to_owned(), new_ref: new_name.to_owned() },
                ]
            }
    }
}

/// NotionObject is an abstraction of a notion object (like a page or a database).
/// It's used to link together all the files that belong to the same notion object.
/// For example, if a page has a subpage, the page will have a directory with the same name,
/// and they will be linked together by the NotionObject.
pub enum NotionObject {
    Page(NotionObjectInfo, Option<ReplacableReferencesContainer>),
    Database(NotionObjectInfo, NotionDatabaseInfo, Option<ReplacableReferencesContainer>),
    OtherText { path: PathBuf },
    OtherBinary { path: PathBuf },
}

// FACTORY
impl NotionObject {
    /// Returns a list of all NotionObjects.
    pub fn create_objects_from_file_map(all_files: &FileMapByName) -> Vec<NotionObject> {
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
                    path: html_path.unwrap()
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
                        .unwrap_or_else(|| panic!("No space in file name: {}. Maybe it has already been renamed?", key));
                    let name = key[0..last_space_index].to_string();
                    let uuid = key[last_space_index + 1..].to_string();
                    notion_objects.push(NotionObject::Page(
                        NotionObjectInfo::new(
                            page_path,
                            name,
                            uuid,
                            dir_path
                        ),
                        None
                    ));
                }
                // Database file
                // A database file can have an associated html file
                (None, html_file, Some(csv_path), csv_all_path, dir_path) => {
                    let last_space_index = key
                        .rfind(' ')
                        .unwrap_or_else(|| panic!("No space in file name: {}. Maybe it has already been renamed?", key));
                    let name = key[0..last_space_index].to_string();
                    let uuid = key[last_space_index + 1..].to_string();
                    notion_objects.push(NotionObject::Database(
                        NotionObjectInfo::new(
                            csv_path,
                            name,
                            uuid,
                            dir_path
                        ),
                        NotionDatabaseInfo {
                            csv_all_path,
                            html_path: html_file,
                        },
                        None
                    ));
                }
                // A special case of a database with a csv_all file but no csv file and no html or markdown page
                (None, None, None, Some(csv_all_path), Some(dir_path)) => panic!(
                    "--------------------\nDatabase with csv_all file and a directory, but no csv file and no html or markdown page: KEY: {}\n\tCSV_ALL: {:?}\n\tDIR: {:?}\n-> My advice is to simply remove the '_all' at the end of {:?}. I may add proper support for this format later.\n--------------------\n",
                    key,
                    csv_all_path,
                    dir_path,
                    csv_all_path
                ),
                // Directory alone. we dont rename it, so skip it.
                // (All renamable directories are associated with a page or a database)
                (None, None, None, None, Some(_)) => {}
                // Invalid !
                (md_path, html_path, csv_path, csv_all_path, dir_path) => panic!(
                    "Invalid path combination with key [{}]):\n\tMarkdown: {:?}\n\tHTML: {:?}\n\tCSV: {:?}\n\tCSV_ALL: {:?}\n\tDIR: {:?}",
                    key,
                    md_path,
                    html_path,
                    csv_path,
                    csv_all_path,
                    dir_path
                ),
            }
        }

        notion_objects
    }

    /// Returns a map of all NotionObjects by their name (without the UUID)
    pub fn build_map_by_name(notion_objects: Vec<NotionObject>) -> ObjectsMapByName {
        let mut map: ObjectsMapByName = HashMap::new();
        for notion_object in notion_objects {
            let name = notion_object.get_name().to_string();
            map.entry(name).or_default().push(notion_object);
        }
        map
    }
}

// GETTERS-SETTERS
impl NotionObject {
    pub fn get_uuid_or_invalid(&self) -> &str {
        match self {
            NotionObject::Page(info, ..) | NotionObject::Database(info, ..) => &info.uuid,
           _ => "00000000000000000000000000000000"
        }
    }

    /// Is there a directory associated with this object?
    pub fn has_dir(&self) -> bool {
        match self {
            NotionObject::Page(info, ..) | NotionObject::Database(info, ..) => info.dir_path.is_some(),
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. } => false,
        }
    }

    /// Gets the path to the directory associated with this object.
    pub fn get_dir(&self) -> Option<&PathBuf> {
        match self {
            NotionObject::Page(info, ..) | NotionObject::Database(info, ..) => info.dir_path.as_ref(),
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. } => None,
        }
    }

    /// Gets the name of the object (without the UUID).
    pub fn get_name(&self) -> &str {
        match self {
            NotionObject::Page(info, ..) | NotionObject::Database(info, ..) => &info.name,
            NotionObject::OtherText { path, .. } | NotionObject::OtherBinary { path, .. } => {
                path.file_stem().unwrap().to_str().unwrap()
            }
        }
    }

    /// Returns true if this object is a page or a database.
    pub fn is_page_or_dataset(&self) -> bool {
        matches!(self, NotionObject::Page(..) | NotionObject::Database(..))
    }

    /// Sets new_name for renamable objects, ie pages and databases.
    pub fn accept_new_name(&mut self, new_name: String) {
        match self {
            NotionObject::Page(info, replace_strings) | NotionObject::Database(info, _, replace_strings) => {
                info.new_name = Some(new_name.clone());

                assert!(replace_strings.is_none());

                *replace_strings = Some(ReplacableReferencesContainer::build_from_name(&info.old_name, &new_name));
            }
            // 'Other' files don't have to be renamed
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. } => {}
        }
    }
    
    pub fn get_path(&self) -> &PathBuf {
        match self {
            NotionObject::Page(info, ..) | NotionObject::Database(info, ..) => &info.path,
            NotionObject::OtherText { path, .. } | NotionObject::OtherBinary { path, .. } => path,
        }
    }
    
    pub fn get_relative_path(&self, base: &PathBuf) -> Option<String> {
        match self {
            NotionObject::Page(info, ..) | NotionObject::Database(info, ..) => {
                let relative_path = pathdiff::diff_paths(self.get_path(), base)?;
                let relative_path_with_new_name = relative_path.to_str()?.replace(&info.old_name, info.new_name.as_ref()?);
                Some(relative_path_with_new_name)
            },
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. } => None,
        }
    }
}

impl NotionObject {

    pub fn get_old_and_new_paths(&self) -> (PathBuf, PathBuf) {
        let old_path = self.get_path();
        match self {
            NotionObject::Page(obj_info, ..) | NotionObject::Database(obj_info, ..) => {
                let new_path = old_path
                    .with_file_name(obj_info.new_name.as_ref().unwrap())
                    .with_extension(old_path.extension().unwrap());

                (old_path.to_owned(), new_path)
            }
            _ => panic!("non-page, non-database object wont be renamed"),
        }
    }

    pub fn get_old_and_new_csv_all_paths(&self) -> Option<(PathBuf, PathBuf)> {
        if let NotionObject::Database(
            obj_info,
            NotionDatabaseInfo {
                csv_all_path: Some(old_csv_all_path),
                ..
            },
            ..
        ) = self
        {
            // Rename also the csv_all
            let new_csv_all_path = old_csv_all_path
                .with_file_name(obj_info.new_name.as_ref().unwrap().to_owned() + "_all")
                .with_extension(old_csv_all_path.extension().unwrap());

            return Some((old_csv_all_path.to_owned(), new_csv_all_path));
        }

        None
    }

    pub fn get_old_and_new_html_paths(&self) -> Option<(PathBuf, PathBuf)> {
        if let NotionObject::Database(
            obj_info,
            NotionDatabaseInfo {
                html_path: Some(old_html_path),
                ..
            },
            ..
        ) = self
        {
            // Rename also the html
            let new_html_path = old_html_path
                .with_file_name(obj_info.new_name.as_ref().unwrap())
                .with_extension(old_html_path.extension().unwrap());

            return Some((old_html_path.to_owned(), new_html_path));
        }

        None
    }

    pub fn get_old_and_new_dir_paths(&self) -> (PathBuf, PathBuf) {
        let old_dir_path = self.get_dir().unwrap();
        match self {
            NotionObject::Page(obj_info, ..) | NotionObject::Database(obj_info, ..) => {
                let new_dir_path = old_dir_path.with_file_name(obj_info.new_name.as_ref().unwrap());
                (old_dir_path.to_owned(), new_dir_path)
            }
            _ => panic!("non-page, non-database object dont have a directory"),
        }
    }
}