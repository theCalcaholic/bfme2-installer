use std::collections::VecDeque;
use md5::{Digest, Md5};
use md5::digest::Output;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use iced_futures::{BoxStream, futures};


const BUFFER_SIZE: usize = 1024;

fn md5sum<D: Digest + Default, R: Read>(reader: &mut R) -> Result<Output<D>, String> {
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

fn generate_files_list(path: PathBuf) -> Vec<String> {
    let install_path = &path;
    install_path.read_dir()
        .expect(format!("Error: Could not read game directory!").to_string().deref())
        .filter_map(|r| r.ok())
        .map(|f| match f.path().is_dir() {
            true => generate_files_list(f.path().to_path_buf()),
            false => Vec::from([String::from(f.path().to_str().unwrap())])
        })
        .flatten()
        .collect()
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
    pub path: String,
}

impl<H, I, T> iced_native::subscription::Recipe<H, I> for ChecksumGenerator<T>
    where
        T: 'static + Hash + Copy + Send,
        H: Hasher
{
    type Output = (T, Progress);

    fn hash(&self, state: &mut H) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
        self.id.hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<I>) -> BoxStream<Self::Output> {
        let id = self.id;

        println!("Retrieving list of files...");
        let file_queue = generate_files_list(PathBuf::from(&self.path));
        let initial_state = ChecksumState::Start(
            (&*self.path).parse().unwrap(),
            Vec::new(),
            VecDeque::from(file_queue.clone()),
            0, file_queue.iter().count() as u32);

        Box::pin(futures::stream::unfold(
            initial_state,
            move |state| async move {
                match state {
                    ChecksumState::Start(install_path, checksums, file_queue, count, total) => {
                        Some((
                            (id, Progress::Generating(0.0)),
                            ChecksumState::Generating(install_path,checksums, file_queue, count, total)
                        ))
                    },
                    ChecksumState::Generating(install_path, mut checksums, mut file_queue, count, total) => {
                        match file_queue.pop_front() {
                            Some(file_path) => {
                                println!("Calculating md5 sum for {}", file_path);
                                if file_path.ends_with("checksums.txt") {
                                    return Some((
                                        (id, Progress::Generating((count + 1) as f32 * 100.0 / (total as f32))),
                                        ChecksumState::Generating(
                                            install_path,
                                            checksums,
                                            file_queue,
                                            count + 1,
                                            total)
                                    ));
                                }
                                match calculate_hash(PathBuf::from(&file_path)) {
                                    Ok(cs) => {
                                        checksums.push(format!("{}|{}", file_path, cs));
                                        let state =
                                            ChecksumState::Generating(
                                                install_path,
                                                checksums,
                                                file_queue,
                                                count + 1,
                                                total);
                                        Some((
                                            (id, Progress::Generating((count + 1) as f32 * 100.0 / (total as f32))),
                                            state
                                        ))
                                    },
                                    Err(e) => Some((
                                        (id, Progress::Errored),
                                        ChecksumState::Finished
                                    ))
                                }
                            },
                            None => {
                                File::create(PathBuf::from(install_path).join("checksums.txt"))
                                    .unwrap()
                                    .write(checksums.join("\n").as_bytes());
                                return Some((
                                    (id, Progress::Finished),
                                    ChecksumState::Finished
                                ))
                            }

                        }
                    },
                    ChecksumState::Finished => {
                        let _: () = iced::futures::future::pending().await;
                        None
                    }
                }
            }
        ))
    }
}

#[derive(Debug, Clone)]
enum ChecksumState {
    Start(String, Vec<String>, VecDeque<String>, u32, u32),
    Generating(String, Vec<String>, VecDeque<String>, u32, u32),
    Finished
}

#[derive(Debug, Clone)]
pub enum Progress {
    Generating(f32),
    Finished,
    Errored
}