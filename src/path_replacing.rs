use std::{collections::HashSet, fs};

use indicatif::ProgressIterator;

use crate::{constants::PROGRESS_BAR_STYLE, notion_object::{NotionObject, ObjectsMapByName}};

/// Find a new name for objects.
/// ASSUMPTION: No directory can exist without a page or a database.
/// This assumption has been checked in `objects_from_map`, which makes sure either a page or a database exists for each entry.
pub fn resolve_new_names(all_objects_by_name: &mut ObjectsMapByName) {
    for (name, objects) in all_objects_by_name {
        if objects.len() == 1 {
            // This object is the only one to want this name, so we can use it
            objects[0].accept_new_name(name.clone());
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

        // Sort by uuid to ensure determinism
        objects.sort_by(|left_obj, right_obj| left_obj.get_uuid_or_invalid().cmp(&right_obj.get_uuid_or_invalid()));
        
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
                    obj.accept_new_name(
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

/// Renames all files associated with all given objects.
/// Associated files are the csv_all and the html files for databases. NOT the directories.
pub fn rename_objects_files(all_objects: &Vec<&NotionObject>, is_test: bool) {
    for object in all_objects
        .iter()
        .progress_with_style(PROGRESS_BAR_STYLE.clone())
        .filter(|obj| obj.is_page_or_dataset())
    {
        let (old_path, new_path) = object.get_old_and_new_paths();
        
        if !is_test  {
            fs::rename(old_path, new_path).unwrap(); // Should not panic
        }

        if let Some((old_csv_all_path, new_csv_all_path)) = object.get_old_and_new_csv_all_paths()
        {
            if !is_test{
                fs::rename(old_csv_all_path, new_csv_all_path).unwrap(); // Should not panic
            }
        }

        if let Some((old_html_path, new_html_path)) = object.get_old_and_new_html_paths() {
            if !is_test{
                fs::rename(old_html_path, new_html_path).unwrap(); // Should not panic
            }
        }
    }
}

/// Renames all directories associated with all given objects.
/// Sorts the directories by their depth (deepest first) to avoid conflicts.
/// Indeed, renaming a parent directory first would invalidate the child path.
pub fn rename_directories(all_objects: &Vec<&NotionObject>, is_test: bool) {
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
        .progress_with_style(PROGRESS_BAR_STYLE.clone())
    {
        let (old_dir_path, new_dir_path) = object.get_old_and_new_dir_paths();

        if !is_test {
            fs::rename(old_dir_path, new_dir_path).unwrap(); // Should not panic
        }
    }
}