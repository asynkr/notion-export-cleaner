use std::borrow::Cow;
use std::cmp::max;
use std::fs;
use std::ops::Range;
use std::path::PathBuf;

use indicatif::ProgressIterator;

use crate::constants::{NOTION_LINK_MARKER, PROGRESS_BAR_STYLE};
use crate::file_type::FileType;
use crate::notion_object::NotionObject;

struct ReplaceDescriptor<'a> {
    byte_range: Range<usize>,
    new_text: Cow<'a, str>
}

impl<'a> ReplaceDescriptor<'a> {
    fn new_from_borrowed(start: usize, end:usize, new_text: &'a str) -> Self {
        assert!(start < end);
        Self { byte_range: (start..end), new_text: Cow::Borrowed(new_text) }
    }

    fn new_from_owned(start: usize, end:usize, new_text: String) -> Self {
        assert!(start < end);
        Self { byte_range: (start..end), new_text: Cow::Owned(new_text) }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum RenameRefsInFileError {
    #[error("The uuid {uuid} ({new_name}) remains in renamed content:\n\tFound in: '...{window_where_uuid_appears}...'\n\tStrings replaced: {looked_for:?}")]
    RefRemainingInFile{
        uuid: String,
        new_name: String,
        window_where_uuid_appears: String,
        looked_for: Vec<String>,
    },
}

fn prepare_rename_refs_in_content<'a>(file_contents: &str, file_path: &PathBuf, all_objects: &Vec<&'a NotionObject>) -> Vec<ReplaceDescriptor<'a>> {
    const REPLACE_DESCRIPTORS_ESTIMATED_AMOUNT: usize = 2048;
    let mut replace_descriptors: Vec<ReplaceDescriptor> = Vec::with_capacity(REPLACE_DESCRIPTORS_ESTIMATED_AMOUNT);

    let notion_links_markers: Vec<Range<usize>> = file_contents.match_indices(NOTION_LINK_MARKER)
        .filter_map(|(notion_link_start, excerpt)| -> Option<Range<usize>> {
            const HTTPS_WINDOW_BYTE_SIZE: usize = 30; // "https://www." is 12 bytes long, make room for a bit more
            const UUID_BYTE_SIZE: usize = 36;
            const AFTER_LINK_WINDOW_BYTE_SIZE: usize = 60; // /?arg=smthg is 11 bytes long, make room for a bit more

            let window_start = if notion_link_start <= HTTPS_WINDOW_BYTE_SIZE { 0 } else { file_contents.floor_char_boundary(notion_link_start - HTTPS_WINDOW_BYTE_SIZE) };
            let window_end = file_contents.ceil_char_boundary(notion_link_start + excerpt.len() + UUID_BYTE_SIZE + AFTER_LINK_WINDOW_BYTE_SIZE);

            if let Some(last_open_par_index) = file_contents[window_start..notion_link_start].find('(').and_then(|found| Some(window_start + found)) {
                if let Some(first_close_par_index) = file_contents[notion_link_start..window_end].find(')').and_then(|found| Some(notion_link_start + found)) {
                    let range_without_parenthesis = (last_open_par_index + 1)..first_close_par_index;
                    assert!(file_contents[range_without_parenthesis.clone()].contains(NOTION_LINK_MARKER));
                    return Some(range_without_parenthesis);
                }
            }

            None
        })
        .collect();

    for object in all_objects {

        let (obj_info, replacable_refs) = match object {
            NotionObject::Page(obj_info, replacable_refs) | NotionObject::Database(obj_info, _, replacable_refs) => {
                (obj_info, replacable_refs)
            }
            _ => continue,
        };

        // Most refs are simple find & replace
        if let Some(replacable_refs) = replacable_refs {
            for replacable_ref in &replacable_refs.global_references {
                file_contents.match_indices(&replacable_ref.old_ref).for_each(|(start, m)| {
                    replace_descriptors.push(ReplaceDescriptor::new_from_borrowed(start, start + m.len(), &replacable_ref.new_ref));
                });
            }
        }

        // Some refs are Notion paths
        // https://www.notion.so/uuid?arg=smthg
        // We will replace them with relative disk paths
        if let Some(relative_path_with_new_name) = object.get_relative_path(file_path) {
            for range in &notion_links_markers {
                let window = &file_contents[range.clone()];
                if window.contains(&obj_info.uuid) {
                    replace_descriptors.push(ReplaceDescriptor::new_from_owned(range.start, range.end, relative_path_with_new_name.clone()));
                }
            }
        }
    }

    replace_descriptors
}

/// Renames all references to all objects in given file.
fn do_rename_refs_in_content(file_contents: &str, mut descriptors: Vec<ReplaceDescriptor>) -> Option<String>
{
    if descriptors.is_empty() {
        return None;
    }

    descriptors.sort_by_key(|d| d.byte_range.start);

    for desc_index in (1..descriptors.len()).rev() {
        let descriptor = &descriptors[desc_index];
        let previous_descriptor = &descriptors[desc_index - 1];

        if descriptor.byte_range.start < previous_descriptor.byte_range.end {
            descriptors.remove(desc_index);
        }
    }

    let mut new_contents = String::with_capacity(file_contents.len());

    let mut last_end = 0;
    for ReplaceDescriptor { byte_range, new_text } in &descriptors {
        new_contents.push_str(&file_contents[last_end..byte_range.start]);
        new_contents.push_str(&new_text);
        last_end = byte_range.end
    }

    new_contents.push_str(&file_contents[last_end..]);

    Some(new_contents)
}

fn validate_rename_refs_in_content(new_file_contents: &str, all_objects: &Vec<&NotionObject>) -> Vec<RenameRefsInFileError> {
    let mut errors = vec![];
    for obj in all_objects {
        match obj {
            NotionObject::Page(obj_info, replace_strings) | NotionObject::Database(obj_info, _, replace_strings) => {
                if let Some(uuid_byte_index) = new_file_contents.find(obj_info.uuid.as_str()) {
                    // UUID found! raise error

                    let window_byte_offset = max(30, obj_info.name.len() * 2); // Arbitrary offset, to accomodate at least more than 1 instance of the name.
                    let uuid_size = obj_info.uuid.len();
                    let window_where_uuid_appears_begin = if uuid_byte_index > window_byte_offset {
                        new_file_contents.floor_char_boundary(uuid_byte_index - window_byte_offset)
                    } else {
                        0
                    };
                    let window_where_uuid_appears_end = new_file_contents.ceil_char_boundary(uuid_byte_index + uuid_size + window_byte_offset);
                    let window_where_uuid_appears = &new_file_contents[window_where_uuid_appears_begin..window_where_uuid_appears_end];

                    errors.push(RenameRefsInFileError::RefRemainingInFile {
                        uuid: obj_info.uuid.clone(),
                        new_name: obj_info.new_name.to_owned().unwrap_or_default(),
                        // window does not need to be precise
                        window_where_uuid_appears: window_where_uuid_appears.to_owned(),
                        looked_for: match replace_strings {
                            Some(replace_strings_container) => {
                                replace_strings_container.global_references
                                    .iter()
                                    .map(|c| c.old_ref.clone())
                                    .collect()
                            },
                            None => vec![]
                        },
                    });
                }
            },
            _ => continue,
        };
    }

    errors
}

/// Renames all references to all objects in all given files.
pub fn rename_refs_in_all_files(all_files: &Vec<&FileType>, all_objects: &Vec<&NotionObject>, is_test: bool) {
    for file in all_files
        .iter()
        .progress_with_style(PROGRESS_BAR_STYLE.clone())
        .filter(|ft| ft.is_readable_type())
    {
        let path = file.get_path();

        let old_contents = fs::read_to_string(path).unwrap(); // Should not panic, file should be readable
        let replace_descriptors = prepare_rename_refs_in_content(&old_contents, path, all_objects);
        if let Some(new_content) = do_rename_refs_in_content(&old_contents, replace_descriptors) {
            
            let errors_encountered = validate_rename_refs_in_content(&new_content, all_objects);
    
            for error in errors_encountered {
                match error {
                    RenameRefsInFileError::RefRemainingInFile{ .. } => {
                        if path.ends_with("index.html") {
                            // uuid is expected to appear in index.html. It's not a failing renaming.
                            continue;
                        }
    
                        println!("Warning: non-fatal problem found while renaming references in {:?}:\n\t{}", path, error);
                    }
                }
            }
    
            if !is_test {
                fs::write(path,new_content).unwrap(); // Should not panic, file should be writable
            }
        }
    }
}