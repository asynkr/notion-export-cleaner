use indicatif::ProgressIterator;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

use crate::file_type::FileType;

/// The `index.html` file has a `index` key.
/// It shouldn't be totally ignored because it has content to be modified,
/// but it shouldn't be renamed. So it shouldn't be a Page object.
const INDEX_KEY: &str = "index";

type ObjectsMapByName = HashMap<String, Vec<NotionObject>>;

/// Stores the info of a notion object.
/// These info are common for pages and databases.
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
    /// Best case (this file is the only one to want this name): its `name`.
    /// Worst case (multiple candidates): its `name` + space + a number.
    new_name: Option<String>,
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

/// NotionObject is an abstraction of a notion object (like a page or a database).
/// It's used to link together all the files that belong to the same notion object.
/// For example, if a page has a subpage, the page will have a directory with the same name,
/// and they will be linked together by the NotionObject.
pub enum NotionObject {
    Page(NotionObjectInfo),
    Database(NotionObjectInfo, NotionDatabaseInfo),
    OtherText { path: PathBuf },
    OtherBinary { path: PathBuf },
}

// FACTORY
impl NotionObject {
    /// Returns a list of all NotionObjects.
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
                    notion_objects.push(NotionObject::Database(
                        NotionObjectInfo {
                            path: csv_path,
                            name,
                            uuid,
                            dir_path,
                            new_name: None,
                        },
                        NotionDatabaseInfo {
                            csv_all_path,
                            html_path: html_file,
                        }
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
    fn get_name_uuid(&self) -> String {
        match self {
            NotionObject::Page(info) | NotionObject::Database(info, _) => {
                format!("{} {}", info.name, info.uuid)
            }
            NotionObject::OtherText { path } | NotionObject::OtherBinary { path } => {
                path.file_stem().unwrap().to_str().unwrap().to_string()
            }
        }
    }

    /// Is there a directory associated with this object?
    fn has_dir(&self) -> bool {
        match self {
            NotionObject::Page(info) | NotionObject::Database(info, _) => info.dir_path.is_some(),
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. } => false,
        }
    }

    /// Gets the path to the directory associated with this object.
    fn get_dir(&self) -> Option<&PathBuf> {
        match self {
            NotionObject::Page(info) | NotionObject::Database(info, _) => info.dir_path.as_ref(),
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. } => None,
        }
    }

    /// Gets the name of the object (without the UUID).
    fn get_name(&self) -> &str {
        match self {
            NotionObject::Page(info) | NotionObject::Database(info, _) => &info.name,
            NotionObject::OtherText { path } | NotionObject::OtherBinary { path } => {
                path.file_stem().unwrap().to_str().unwrap()
            }
        }
    }

    /// Sets new_name for renamable objects, ie pages and databases.
    fn try_set_new_name(&mut self, new_name: String) {
        match self {
            NotionObject::Page(info) | NotionObject::Database(info, _) => {
                info.new_name = Some(new_name)
            }
            // 'Other' files don't have to be renamed
            NotionObject::OtherText { .. } | NotionObject::OtherBinary { .. } => {}
        }
    }

    fn get_path(&self) -> &PathBuf {
        match self {
            NotionObject::Page(info) | NotionObject::Database(info, _) => &info.path,
            NotionObject::OtherText { path } | NotionObject::OtherBinary { path } => path,
        }
    }

    /// Returns true if this object is a page or a database.
    fn is_page_or_dataset(&self) -> bool {
        matches!(self, NotionObject::Page(_) | NotionObject::Database(_, _))
    }
}

// BUSINESS LOGIC
impl NotionObject {
    /// Find a new name for this object.
    /// ASSUMPTION: No directory can exist without a page or a database.
    /// This assumption has been checked in `objects_from_map`, which makes sure either a page or a database exists for each entry.
    pub fn find_new_names(all_objects_by_name: &mut ObjectsMapByName) {
        for (name, objects) in all_objects_by_name {
            if objects.len() == 1 {
                // This object is the only one to want this name, so we can use it
                objects[0].try_set_new_name(name.clone());
                continue;
            }

            // paths that we expect after renaming the files (not touching the directories)
            // e.g. for file "/parent page 15278/page 579632.md", the expected path is "/parent page 15278/page.md"
            // it's used to see if there are conflicts that need a suffix
            let paths_after_files_renamed = objects
                .iter()
                .map(|obj| {
                    if !obj.is_page_or_dataset() {
                        // there's no uuid in these files
                        // they wont be renamed!
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
                match paths_after_files_renamed[i].as_ref() {
                    // This object is not a page or a database, so we don't need to rename it
                    None => continue,
                    Some(desired_path) => {
                        let mut desired_path = desired_path.clone();

                        let mut add = 1;
                        while new_paths_seen.contains(&desired_path) {
                            // This exact path already exists, so we need to add a number to the end of the name
                            desired_path = desired_path
                                .with_file_name(
                                    format!("{} {}", name, add), // No extension at this point, with_file_name removes it
                                )
                                .with_extension(desired_path.extension().unwrap_or_default());
                            add += 1;
                        }

                        // pfew! exiting the loop, we found a name that doesn't conflict with any other
                        new_paths_seen.insert(desired_path.clone());

                        // Sets the new name!
                        obj.try_set_new_name(
                            desired_path
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

    /// Renames all references to this object in a given file.
    fn rename_refs_in_file(&self, file_contents: &str) -> String {
        let mut new_contents: String = file_contents.to_string();

        let old_name = self.get_name_uuid();
        let new_name = match self {
            NotionObject::Page(obj_info) | NotionObject::Database(obj_info, _) => {
                obj_info.new_name.as_ref().unwrap().as_str()
            }
            _ => panic!("non-page, non-database object wont be renamed"),
        };

        // Some of these references are url-encoded
        let old_name_url_encoded = urlencoding::encode(&old_name).into_owned();
        let new_name_url_encoded = urlencoding::encode(new_name).into_owned();

        // Some of these references are html-encoded
        let old_name_html_encoded = html_escape::encode_safe(&old_name).into_owned();
        let new_name_html_encoded = html_escape::encode_safe(new_name).into_owned();

        // Renames all references
        // files, csv_all, directories, etc
        new_contents = new_contents.replace(&old_name, new_name);
        new_contents = new_contents.replace(&old_name_url_encoded, &new_name_url_encoded);
        new_contents = new_contents.replace(&old_name_html_encoded, &new_name_html_encoded);

        new_contents
    }

    /// Renames all references to all objects in all given files.
    pub fn rename_refs_in_all_files(all_files: Vec<&FileType>, all_objects: Vec<&NotionObject>) {
        for file in all_files
            .iter()
            .progress()
            .filter(|ft| ft.is_readable_type())
        {
            let path = file.get_path();

            let old_contents = fs::read_to_string(path).unwrap(); // Should not panic, file should be readable
            let mut new_contents = old_contents.clone();

            for object in all_objects.iter().filter(|obj| obj.is_page_or_dataset()) {
                new_contents = object.rename_refs_in_file(&new_contents);
            }

            if old_contents != new_contents {
                fs::write(path, new_contents).unwrap(); // Should not panic, file should be writable
            }
        }
    }

    /// Rename the file and its associated files if any.
    /// Associated files are the csv_all and the html file for databases. NOT the directory.
    fn rename_object_files(&self) {
        let old_path = self.get_path();
        match self {
            NotionObject::Page(obj_info) | NotionObject::Database(obj_info, _) => {
                let new_path = old_path
                    .with_file_name(obj_info.new_name.as_ref().unwrap())
                    .with_extension(old_path.extension().unwrap());
                fs::rename(old_path, new_path).unwrap(); // Should not panic
            }
            _ => panic!("non-page, non-database object wont be renamed"),
        }

        if let NotionObject::Database(
            obj_info,
            NotionDatabaseInfo {
                csv_all_path: Some(old_csv_all_path),
                ..
            },
        ) = self
        {
            // Rename also the csv_all
            let new_csv_all_path = old_csv_all_path
                .with_file_name(obj_info.new_name.as_ref().unwrap().to_owned() + "_all")
                .with_extension(old_csv_all_path.extension().unwrap());

            fs::rename(old_csv_all_path, new_csv_all_path).unwrap(); // Should not panic
        }

        if let NotionObject::Database(
            obj_info,
            NotionDatabaseInfo {
                html_path: Some(old_html_path),
                ..
            },
        ) = self
        {
            // Rename also the html
            let new_html_path = old_html_path
                .with_file_name(obj_info.new_name.as_ref().unwrap())
                .with_extension(old_html_path.extension().unwrap());

            fs::rename(old_html_path, new_html_path).unwrap(); // Should not panic
        }
    }

    /// Renames all files associated with all given objects.
    /// Associated files are the csv_all and the html files for databases. NOT the directories.
    pub fn rename_objects_files(all_objects: Vec<&NotionObject>) {
        for object in all_objects
            .iter()
            .progress()
            .filter(|obj| obj.is_page_or_dataset())
        {
            object.rename_object_files();
        }
    }

    /// Renames the directory associated with this object.
    fn rename_dir(&self) {
        let old_dir_path = self.get_dir().unwrap();
        match self {
            NotionObject::Page(obj_info) | NotionObject::Database(obj_info, _) => {
                let new_dir_path = old_dir_path.with_file_name(obj_info.new_name.as_ref().unwrap());
                fs::rename(old_dir_path, new_dir_path).unwrap(); // Should not panic
            }
            _ => panic!("non-page, non-database object dont have a directory"),
        }
    }

    /// Renames all directories associated with all given objects.
    /// Sorts the directories by their depth (deepest first) to avoid conflicts.
    /// Indeed, renaming a parent directory first would invalidate the child path.
    pub fn rename_directories(all_objects: Vec<&NotionObject>) {
        let mut all_objects_sorted_by_dir_path_len_highest_first = all_objects
            .iter()
            .filter(|obj| obj.has_dir())
            .copied()
            .collect::<Vec<&NotionObject>>();
        all_objects_sorted_by_dir_path_len_highest_first
            .sort_by_key(|obj| obj.get_dir().unwrap().components().count());
        all_objects_sorted_by_dir_path_len_highest_first.reverse();

        for object in all_objects_sorted_by_dir_path_len_highest_first
            .iter()
            .progress()
        {
            object.rename_dir();
        }
    }
}
