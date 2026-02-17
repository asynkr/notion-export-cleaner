use std::{collections::HashMap, fs, path::PathBuf};

pub type FileMapByName = HashMap<String, Vec<FileType>>;

/// Stores the path and name + UUID of a file or directory.
#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name_uuid: String,
}

/// FileType is an abstraction of a real file or directory in the input directory.
#[derive(Debug)]
pub enum FileType {
    Markdown(FileInfo),
    Html(FileInfo),
    Csv(FileInfo),
    CsvAll(FileInfo),
    Dir(FileInfo),
    OtherTxt(PathBuf),
    OtherBin(PathBuf),
}

impl FileType {
    /// For most files or dirs, this is the name + space + UUID.
    /// For files that are *not* markdown, or html, or csv, it's the name of the file without the extension.
    /// For a csv_all file, it's the name + space + UUID (WITHOUT the _all).
    /// The file key is used to group files that should be renamed together.
    pub fn get_file_key(&self) -> &str {
        match self {
            FileType::Markdown(file_info)
            | FileType::Html(file_info)
            | FileType::Csv(file_info)
            | FileType::CsvAll(file_info)
            | FileType::Dir(file_info) => &file_info.name_uuid,
            FileType::OtherTxt(path) | FileType::OtherBin(path) => {
                path.file_stem().unwrap().to_str().unwrap()
            }
        }
    }

    pub fn is_readable_type(&self) -> bool {
        matches!(
            self,
            FileType::Markdown(_)
                | FileType::Html(_)
                | FileType::Csv(_)
                | FileType::CsvAll(_)
                | FileType::OtherTxt(_)
        )
        // Bin and Dir are not readable
    }

    pub fn get_path(&self) -> &PathBuf {
        match self {
            FileType::Markdown(file_info)
            | FileType::Html(file_info)
            | FileType::Csv(file_info)
            | FileType::CsvAll(file_info)
            | FileType::Dir(file_info) => &file_info.path,
            FileType::OtherTxt(path) | FileType::OtherBin(path) => path,
        }
    }
}

impl From<PathBuf> for FileType {
    fn from(path: PathBuf) -> Self {
        let name_uuid = path.file_stem().unwrap().to_str().unwrap();
        let file_info = FileInfo {
            path: path.clone(),
            name_uuid: name_uuid.to_string(),
        };

        if path.is_dir() {
            return FileType::Dir(file_info);
        }

        let extension = path.extension().unwrap().to_str().unwrap();

        match extension {
            "md" => FileType::Markdown(file_info),
            "html" => FileType::Html(file_info),
            "csv" => {
                let all_suffix = "_all";
                if file_info.name_uuid.ends_with(all_suffix) {
                    let file_info = FileInfo {
                        path: file_info.path,
                        name_uuid: file_info.name_uuid
                            [0..file_info.name_uuid.len() - all_suffix.len()]
                            .to_string(),
                    };
                    FileType::CsvAll(file_info)
                } else {
                    FileType::Csv(file_info)
                }
            }
            // The file is not one of the supported extension. We will try to read it as a text file.
            // Because if it is a text file, renaming links inside it may be necessary.
            _ => match fs::read_to_string(path) {
                Ok(_) => FileType::OtherTxt(file_info.path),
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                    // This file is not readable.
                    FileType::OtherBin(file_info.path)
                }
                Err(e) => panic!("Unexpected error reading file: {}", e),
            },
        }
    }
}
