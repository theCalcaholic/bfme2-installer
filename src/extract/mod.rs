use std::borrow::Borrow;
use std::fs::{File, read_link};
use std::path::{Path, PathBuf};
use flate2::read::{GzDecoder};
use tar::{Archive, Entry, Entries};
use std::hash::{Hash, Hasher};
use iced_futures::futures;
use std::collections::VecDeque;

use crate::common::{Message, InstallationProgress};

// pub fn extract<I: 'static + Hash + Copy + Send, S: ToString>(id: I, from: S, to: S
// ) -> iced::Subscription<(I, Progress)> {
//     iced::Subscription::from_recipe(Extraction {
//         id,
//         from: from.to_string(),
//         to: to.to_string(),
//         progress: 0.0
//     })
// }

pub struct Extraction<I> {
    pub id: I,
    pub from: VecDeque<String>,
    pub to: String
}

impl<H, I, T> iced_native::subscription::Recipe<H, I> for Extraction<T>
where
    T: 'static + Hash + Copy + Send,
    H: Hasher,
{
    type Output = (T, InstallationProgress);

    fn hash(&self, state: &mut H) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
        self.id.hash(state);
    }

    fn stream(self: Box<Self>, _input: iced_futures::BoxStream<I>)
        -> iced_futures::BoxStream<Self::Output>
    {
        let id = self.id;
        let total = self.from.iter().count() as f32;

        Box::pin(futures::stream::unfold(
            // initial value for future
            ExtractionState::Start(self.to, self.from, 0, total),
            // closure executed in future
            move |state| async move {
                match state {
                    ExtractionState::Start(target, mut archives, count, total) => {
                        Some((
                            (id, InstallationProgress::Extracting(count as f32 * total * 100.0,
                                          String::from(archives.front().unwrap_or(&("Done".to_string()))))),
                            ExtractionState::Extracting(target, archives, count, total)
                            ))
                    },
                    ExtractionState::Extracting(target, mut archives, count, total) => {

                        match archives.pop_front() {
                            Some(path) => {
                                let tar = File::open(&path).unwrap();
                                let decompressed = GzDecoder::new(tar);
                                let mut archive = Archive::new(decompressed);
                                println!("Unpacking '{}' to '{}'", path, &target);
                                match archive.unpack(PathBuf::from(&target)) {
                                    Ok(()) => {
                                        Some((
                                            (id, InstallationProgress::Extracting((count + 1) as f32 * 100.0 / total,
                                                                    String::from(archives.front().unwrap_or(&("Done".to_string()))))),
                                            ExtractionState::Extracting(target,
                                                                        archives,
                                                                        count + 1,
                                                                        total)
                                        ))
                                    },
                                    Err(e) => {
                                        println!("ERROR: {}", e.to_string());
                                        Some((
                                            (id, InstallationProgress::Errored(e.to_string())),
                                            ExtractionState::Finished
                                        ))
                                    }
                                }
                            },
                            None => Some((
                                (id, InstallationProgress::Finished),
                                ExtractionState::Finished
                            ))
                        }
                    },
                    ExtractionState::Finished => {
                        let _: () = iced::futures::future::pending().await;
                        None
                    }
                }
            }

        ))
    }
}

// #[derive(Debug, Clone)]
// pub enum Progress {
//     Started,
//     Advanced(f32, String),
//     Finished,
//     Errored,
// }

pub enum ExtractionState { //<'a, R: Read> {
    // Extracting {
    //     entries: Entries<'a, R>,
    //     total: usize,
    //     extracted: usize
    // },
    Start(String, VecDeque<String>, u8, f32),
    Extracting(String, VecDeque<String>, u8, f32),
    Finished
}