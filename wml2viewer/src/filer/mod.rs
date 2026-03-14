use std::path::Path;

// background prosses

pub enum FileMap {
  FILE(Path), // OS File
  HTTP(str), // http, https
  CLOUD(Path), // cloud drive, google, OneDrive, iCloud, WebDab
  SMB(Path), // smb protcol
  FileList(FileMap), // file list
}

/*
 Recursive Path Search
 Parent Folder -> Child Folder -> child childe Folder -> next child childe -> Next Child folder -> Next Parent Folder
*/
pub fn search_folder(path: &str) -> Vec<FileMap> {

// todo!
}

/*
  reade file list -> expand loader
 */

pub fn search_file_list(path: &str) -> Vec<FileMap> {

// todo!

}