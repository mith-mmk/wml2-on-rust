use std::path::Path;

// async!
// background prosses
// ファイラーモジュールはasyncで動く。ファイル取得が画像表示より遅い場合は一回ロック……

pub enum FileMap {
  FILE(Path), // OS File
  HTTP(str), // http, https
  CLOUD(Path), // cloud drive, google, OneDrive, iCloud, WebDab
  SMB(Path), // smb protcol
  FileList(FileMap), // file list
}
/*
FileList
UTF-8でファイルのリストが並んでいるが

行頭が
@ で始まる場合は、コマンドになる

@load file <- 差し込みファイル
@wait 100 <- 100ms 遅延
@start x, y <- 座標をx,yに設定

*/


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