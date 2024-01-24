use std::{fs, path::PathBuf};

#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name_uuid: String,
}

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
    /// For random files it's the name of the file without the extension.
    /// For a csv_all file, it's the name + space + UUID (WITHOUT the _all).
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
                if file_info.name_uuid.ends_with("_all") {
                    let file_info = FileInfo {
                        path: file_info.path,
                        name_uuid: file_info.name_uuid[0..file_info.name_uuid.len() - 4]
                            .to_string(),
                    };
                    FileType::CsvAll(file_info)
                } else {
                    FileType::Csv(file_info)
                }
            }
            _ => match fs::read_to_string(path) {
                Ok(_) => FileType::OtherTxt(file_info.path),
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                    FileType::OtherBin(file_info.path)
                }
                Err(e) => panic!("Unexpected error reading file: {}", e),
            },
        }
    }
}
