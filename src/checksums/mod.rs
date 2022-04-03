use std::collections::VecDeque;
use std::ffi::OsString;
use std::iter::Copied;
use iced::Subscription;
use iced_native::subscription;
use md5::{Digest, Md5};
use md5::digest::Output;
use std::io::{Read, Write};
use std::path::{Path, PathBuf, StripPrefixError};
use std::fs::{File, OpenOptions, self};
use std::fs::{read_to_string};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use iced_futures::{BoxStream, futures};
use super::common::{InstallationProgress};


const BUFFER_SIZE: usize = 1024;

pub fn md5sum<D: Digest + Default, R: Read>(reader: &mut R) -> Result<Output<D>, String> {
    let mut sh = D::default();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let n = match reader.read(&mut buffer) {
            Ok(n) => n,
            Err(e) => return Err(e.to_string().to_owned())
        };
        sh.update(&buffer[..n]);
        if n == 0 || n < BUFFER_SIZE {
            break;
        }
    }

    return Ok(sh.finalize())
}

pub fn generate_files_list(path: PathBuf) -> Vec<String> {
    let install_path = &path;

    match read_to_string(&path.join("checksums.txt")) {
        Ok(contents) => {
            contents
                .replace("\r", "")
                .split("\n")
                .filter_map(|s| match String::from(s).split_once("|") {
                    Some((a, b)) => Some((a.to_owned(), b.to_owned())),
                    None => None
                })
                .map(|t| String::from(t.0))
                .map(|file_path| {
                    let p = PathBuf::from(&file_path);
                    match p.strip_prefix(install_path) {
                        Ok(rel_path) => match rel_path.to_owned().into_os_string().into_string() {
                            Ok(rel_path_str) => Ok(rel_path_str),
                            Err(e) => Err(format!("Failed to get string for file: {:?}", e))
                        },
                        Err(e) => Ok(file_path)
                    }
                    // let a = p.strip_prefix(install_path).to_owned();
                })
                // .map(|p| p.to_owned().into_os_string().into_string())
                .collect::<Result<Vec<String>, String>>()
                .expect("Invalid path!")
        },
        Err(_) => {
            install_path.read_dir()
                .expect(format!("Error: Could not read game directory!").to_string().deref())
                .filter_map(|r| r.ok())
                .filter(|f| f.file_name() != "checksums.txt")
                .map(|f| match f.path().is_dir() {
                    true => generate_files_list(f.path().to_path_buf()),
                    false => Vec::from([String::from(f.path().to_str().unwrap())])
                })
                .flatten()
                .map(|file_path| {
                    let p = PathBuf::from(&file_path);
                    match p.strip_prefix(install_path) {
                        Ok(rel_path) => match rel_path.to_owned().into_os_string().into_string() {
                            Ok(rel_path_str) => Ok(rel_path_str),
                            Err(e) => Err(format!("Failed to get string for file: {:?}", e))
                        },
                        Err(e) => Err(e.to_string())
                    }
                    // let a = p.strip_prefix(install_path).to_owned();
                })
                // .map(|p| p.to_owned().into_os_string().into_string())
                .collect::<Result<Vec<String>, String>>()
                .expect("Invalid path!")
        }
    }

}

pub fn write_checksums_file<S>(install_path: S, results: Vec<(String, String)>) -> Result<(), std::io::Error> 
    where S: Into<String> 
{
    let checksum_path = PathBuf::from(&install_path.into()).join("checksums.txt");
    // if checksum_path.exists() {
    //     fs::remove_file(&checksum_path)?
    // }

    let checksums = results.clone().iter()
        .map(|(path, checksum)| format!("{}|{}", path, checksum))
        .collect::<Vec<String>>()
        .join("\n");
    
    println!("Creating {:?}...", checksum_path);
    File::create(checksum_path)?
        .write(checksums.as_bytes())?;
    Ok(())
}

pub fn calculate_hash(path: PathBuf) -> Result<String, String> {
    if path.is_dir() {
        panic!("Cannot calculate hash for directory!")
    }

    let mut file = File::open(&path)
        .expect(format!("Error reading file {}", path.to_str().unwrap()).to_string().deref());
    match md5sum::<Md5, _>(&mut file) {
        Ok(md5) => {
            let md5_str = md5.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<String>>()
                .join("");
            Ok(md5_str)
        },
        Err(e) => Err(e.to_string())
    }

}

pub struct ChecksumGenerator<I> {
    pub id: I,
    pub install_path: String,
    pub path: String,
}


impl<H, I, T> iced_native::subscription::Recipe<H, I> for ChecksumGenerator<T> where T: 'static + Hash + Copy + Send, H: Hasher {
    type Output = (T, InstallationProgress);

    fn hash(&self, state: &mut H) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
        self.id.hash(state);
    }

    fn stream(self: Box<Self>, _input: futures::stream::BoxStream<'static, I>,) -> futures::stream::BoxStream<'static, Self::Output> { 
        let id = self.id;
        Box::pin(futures::stream::unfold(
                State::Start(self.install_path, self.path),
            move |state| {
                process_file(id, state)
            }
        ))
    }
}

async fn process_file<I: Copy>(id: I, state: State) -> Option<((I, InstallationProgress), State)> {
    match state {
        State::Start(install_path, path) => {
            if path.ends_with("checksums.txt") {
                Some(((id.into(), InstallationProgress::Skipped), State::Finished))
            } else {
                match calculate_hash(PathBuf::from(&install_path).join(&path)) {
                    Ok(cs) => {
                        Some(((id, InstallationProgress::ChecksumResult(path, cs)), State::Finished))
                    },
                    Err(e) => Some(((id, InstallationProgress::Errored(e)), State::Finished))
                }
            }

        }
        State::Finished => {
            let _: () = iced::futures::future::pending().await;
            None
        },
    }
}

#[derive(Debug, Clone)]
enum ChecksumState {
    Start(String, Vec<String>, VecDeque<String>, u32, u32),
    Generating(String, Vec<String>, VecDeque<String>, u32, u32),
    Finished
}

#[derive(Debug, Clone)]
enum State {
    Start(String, String),
    Finished
}


#[derive(Debug, Clone)]
pub enum Progress {
    Finished(String, String),
    Errored,
    Skipped
}