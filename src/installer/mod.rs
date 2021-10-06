use iced::{Column, Text, Element, Button, button, TextInput, text_input, Subscription, ProgressBar, progress_bar, Background, Color};
use super::common::{Message, Game};
use super::extract;
use std::hash::{Hash, Hasher};
use std::collections::{HashMap, VecDeque};
use std::fs::{File, read_link};
use std::path::{Path, PathBuf};
use iced::progress_bar::Style;
use crate::extract::Progress;

#[derive(Debug, Clone, Copy)]
pub enum InstallerStep {
    Inactive,
    Configuration,
    Register,
    Download,
    Install,
    Validate
}

#[derive(Debug, Clone, Copy)]
pub enum InstallationProgress {
    Started,
    Advanced(f32),
    Complete,
    Failed
}

impl InstallerStep {

    fn next(&mut self) -> InstallerStep {
        match self {
            InstallerStep::Configuration => InstallerStep::Register,
            InstallerStep::Register => InstallerStep::Download,
            InstallerStep::Download => InstallerStep::Install,
            InstallerStep::Install => InstallerStep::Validate,
            InstallerStep::Validate => InstallerStep::Inactive,
            _ => self.clone()
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallerData {
    pub game: Option<Game>,
    pub path: String,
    pub checksum: Option<String>,
    pub egrc: Option<String>
}

impl InstallerData {
    fn defaults() -> InstallerData {
        return InstallerData {
            game: None,
            path: String::from(""),
            checksum: None,
            egrc: None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Installer {
    pub current_step: InstallerStep,
    pub data: InstallerData,
    button_states: [button::State; 1],
    path_input_state: text_input::State,
    extraction_progress: f32,
    extraction_file: String
}



struct PbStyle {
    error: bool
}

impl progress_bar::StyleSheet for PbStyle {
    fn style(&self) -> Style {

        progress_bar::Style {
            background: Background::Color(Color::new(0.8, 0.8, 0.8, 1.0)),
            bar: Background::Color(match self.error {
                true => Color::new(0.6, 0.0, 0.0, 1.0),
                false => Color::new(0.0, 0.8, 0.0, 1.0)
            }),
            border_radius: 15.0
        }
    }
}

impl Installer {

    pub fn new() -> Installer {
        Installer {
            current_step: InstallerStep::Inactive,
            data: InstallerData::defaults(),
            button_states: [button::State::default()],
            path_input_state: text_input::State::default(),
            extraction_progress: 0.0,
            extraction_file: String::from("NONE")
        }
    }
    pub fn view(&mut self) -> Element<Message> {
        match self.current_step {
            InstallerStep::Configuration => self.config_view(),
            InstallerStep::Install => self.install_view(),
            _ => self.default_view()
        }
    }

    pub fn proceed(&mut self, step: InstallerStep) {

        self.current_step = step;
    }

    pub fn install(&self) -> iced::Subscription<(usize, extract::Progress)> {
        let game_str = match self.data.game {
            Some(Game::BFME2) => "BFME2",
            Some(Game::ROTWK) => "ROTWK",
            None => panic!("No game selected! Abort...")
        };
        let data_path = "/mnt/Games/lan/bfme_install_export";
        let extraction_queue = (0..30)
            .map(|n| format!("{}/{}_{}.tar.gz", data_path, game_str, n))
            .filter(|p| Path::new(p).exists())
            .collect::<Vec<String>>();


        let mut extraction = extract::Extraction {
            id: 0,
            from: VecDeque::from(extraction_queue),
            to: String::from("/home/tobias/projects/bfme2-installer/test")
        };
        iced::Subscription::from_recipe( extraction)
    }

    pub fn on_extraction_progressed(&mut self, update: (usize, extract::Progress)) {
        match update.1 {
            extract::Progress::Advanced(percentage, file_name) => {
                self.extraction_progress = percentage;
                self.extraction_file = file_name;
            }
            extract::Progress::Finished => {
                self.extraction_progress = 100.0;
            }
            extract::Progress::Errored => {
                self.extraction_progress = -100.0;
            }
            _ => {
                    self.extraction_progress = 0.0;
            }
        }
    }

    fn config_view(&mut self) -> Element<Message>{
        Column::new()
            .push(Text::new(format!("Installing {:?}", self.data.game.unwrap())).size(20))
            .push(Text::new("Configuration"))
            .push(TextInput::new(&mut self.path_input_state,
                                 "install path", &self.data.path,
                                 Message::InstallerPathUpdate))
            .push(Text::new("Patch Level:"))
            .push(Button::new(&mut self.button_states[0],
                              Text::new("Next"))
                .on_press(Message::InstallerNext(self.current_step.next())))
            .push(Text::new(format!("data:\n{:?}", self.data)))
            .into()
    }

    fn install_view(&mut self) -> Element<Message> {
        println!("Progress: {}", self.extraction_progress);
        let mut view = Column::new()
            .push(Text::new(format!("Installing {:?}", self.data.game.unwrap())).size(20))
            .push(Text::new("Extracting..."))
            .push(ProgressBar::new(0.0..=100.0, self.extraction_progress.abs())
                .style(PbStyle{error: match self.extraction_progress {
                    -100.0 => true,
                    _ => false
                }}))
            .push(Text::new(&self.extraction_file));
        if self.extraction_progress == 100.0 {
            view = view.push(Button::new(&mut self.button_states[0],
                                  Text::new("Next"))
                .on_press(Message::InstallerNext(self.current_step.next())));
        }

            view.into()
    }

    fn default_view(&mut self) -> Element<Message> {
        Column::new()
            .push(Text::new(format!("Installing {:?}", self.data.game.unwrap())).size(20))
            .push(Text::new(format!("{:?}", self.current_step)))
            .push(Button::new(&mut self.button_states[0],
                              Text::new("Next"))
                .on_press(Message::InstallerNext(self.current_step.next())))
            .into()
    }
}
