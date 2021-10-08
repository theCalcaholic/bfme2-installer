use iced::{Column, Text, Element, Button, button, TextInput, text_input, Subscription, ProgressBar, progress_bar, Background, Color};
use super::common::{Message, Game};
use super::extract;
use super::checksums;
use super::reg;
use std::hash::{Hash, Hasher};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use iced::progress_bar::Style;
use crate::extract::Progress;
use handlebars::{Handlebars, RenderError};
use tempfile::{NamedTempFile, tempfile};
use winreg::{RegValue, RegKey};
use winreg::enums::*;
use regex::Regex;

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
            //InstallerStep::Configuration => InstallerStep::Download,
            InstallerStep::Configuration => InstallerStep::Register,
            InstallerStep::Download => InstallerStep::Install,
            InstallerStep::Install => InstallerStep::Register,
            InstallerStep::Validate => InstallerStep::Register,
            InstallerStep::Register => InstallerStep::Inactive,
            _ => self.clone()
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallerData {
    pub game: Option<Game>,
    pub path: String,
    pub checksum: String,
    pub ergc: String
}

impl InstallerData {
    fn defaults() -> InstallerData {
        return InstallerData {
            game: None,
            path: String::default(),
            checksum: String::default(),
            ergc: String::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Installer {
    pub current_step: InstallerStep,
    pub data: InstallerData,
    button_states: [button::State; 1],
    path_input_state: text_input::State,
    ergc_input_state: text_input::State,
    progress: f32,
    progress_message: String
}

struct RegRenderData {
    pub install_path: String,
    pub install_path_shorthand: String,
    pub egrc: Option<String>
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
            ergc_input_state: text_input::State::default(),
            progress: 0.0,
            progress_message: String::from("NONE")
        }
    }
    pub fn view(&mut self) -> Element<Message> {
        match self.current_step {
            InstallerStep::Configuration => self.config_view(),
            InstallerStep::Install => self.install_view(),
            InstallerStep::Validate => self.validate_view(),
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
            to: String::from(&self.data.path)
        };
        iced::Subscription::from_recipe( extraction)
    }

    pub fn on_extraction_progressed(&mut self, update: (usize, extract::Progress)) {
        match update {
            (_, extract::Progress::Advanced(percentage, file_name)) => {
                self.progress = percentage;
                self.progress_message = file_name;
            }
            (_, extract::Progress::Finished) => {
                self.progress = 100.0;
                self.progress_message = String::from("")
            }
            (_, extract::Progress::Errored) => {
                self.progress = -100.0;
            }
            _ => {
                    self.progress = 0.0;
            }
        }
    }

    pub fn on_checksum_progress(&mut self, update: (usize, checksums::Progress)) {
        match update {
            (_, checksums::Progress::Generating(percentage)) => {
                self.progress = percentage;
                self.progress_message = String::from("")
            },
            (_, checksums::Progress::Finished) => {
                self.progress = 100.0;
                self.progress_message = String::from("");
                self.data.checksum = match self.get_checksum() {
                    Ok(result) => result,
                    Err(e) => String::default()
                };
            },
            (_, checksums::Progress::Errored) => {
                self.progress = -100.0;
                self.progress_message = String::from("")
            }
        }
    }

    pub fn generate_checksums(&self) -> iced::Subscription<(usize, checksums::Progress)> {
        let mut checksum_generator = checksums::ChecksumGenerator{
            id: 0,
            path: String::from(&self.data.path)
        };

        iced::Subscription::from_recipe(checksum_generator)
    }

    fn get_checksum(&self) -> Result<String, String> {
        let game_path = PathBuf::from(&self.data.path);


        checksums::calculate_hash(game_path.join("checksums.txt"))
    }

    fn register(data: &InstallerData) -> Result<(), String> {
        let canon_path = PathBuf::from(data.path.as_str())
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let mut reg_data = BTreeMap::new();
        reg_data.insert("install_path", canon_path.clone());
        reg_data.insert("install_path_shorthand", canon_path);
        reg_data.insert("ergc", data.ergc.clone().unwrap());
        let mut handlebars = Handlebars::new();


        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if reg::BFME2.keys.get("HKLM").expect("Unexpected Error!")
            .entries()
            .map(|(key, entries)| {
            entries.entries().map(|(value_name, value)| {
                match value {
                    reg::RegValue::Str(val) => {
                        println!("Write {} to {}", val, key);
                        hklm.create_subkey(key).unwrap().0.set_value(value_name, val)
                    },
                    reg::RegValue::UInt(val) => {
                        println!("Write {} to {}", val, key);
                        hklm.create_subkey(key).unwrap().0.set_value(value_name, val)
                    },
                    reg::RegValue::Template(tmpl) => {
                        let template_name = &*format!("HKLM\\{}\\{}", key, value_name);
                        handlebars.register_template_string(template_name, tmpl);
                        let val = handlebars.render(template_name, &reg_data).unwrap();
                        println!("Write {} to {}", val, key);
                        hklm.create_subkey(key).unwrap().0.set_value(value_name, &val)
                    },
                }
            }).all(|result| result.is_ok())
        }).all(|result| result) {
            Ok(())
        } else {
            Err("An error occurred".parse().unwrap())
        }

    }

    fn config_view(&mut self) -> Element<Message>{
        let mut view = Column::new()
            .push(Text::new(format!("Installing {:?}", self.data.game.unwrap())).size(20))
            .push(Text::new("Configuration"))
            .push(TextInput::new(&mut self.path_input_state,
                                 "install path", &self.data.path,
                                 Message::InstallerConfigUpdate))
            .push(TextInput::new(&mut self.ergc_input_state,
            "activation code", &self.data.ergc, Message::InstallerConfigUpdate))
            .push(Text::new("You can get a valid activation key from here: https://www.youtube.com/watch?v=eWg680bt_es"));
            if PathBuf::from(&self.data.path).is_dir()
                && Regex::new(r"^([A-Z0-9]{4}-?){5}$")
                .is_match(self.data.ergc.replace("-", "").to_uppercase()) {
                view = view.push(Button::new(&mut self.button_states[0],
                                      Text::new("Next"))
                    .on_press(Message::InstallerNext(self.current_step.next())))
            }
            view.push(Text::new(format!("data:\n{:?}", self.data)))
            .into()
    }

    fn progress_view<'a>(game: Game, progress: f32, title: &'a str, progress_message: &'a str) -> Column<'a, Message> {
        println!("Progress: {}", progress);
        Column::new()
            .push(Text::new(format!("Installing {:?}", game)).size(20))
            .push(Text::new(format!("{}...", title)))
            .push(ProgressBar::new(0.0..=100.0, progress.abs())
                .style(PbStyle{error: match progress {
                    -100.0 => true,
                    _ => false
                }}))
            .push(Text::new(progress_message))
    }

    fn install_view(&mut self) -> Element<Message> {
        let mut view = Self::progress_view(self.data.game.unwrap(),
                                                       self.progress,
                                                       "Extracting...",
                                                       self.progress_message.as_str());
        if self.progress == 100.0 {
            view = view.push(Button::new(&mut self.button_states[0],
                                         Text::new("Next"))
                .on_press(Message::InstallerNext(self.current_step.next())));
        }

        view.into()
    }

    fn validate_view(&mut self) -> Element<Message> {
        let mut view = Self::progress_view(self.data.game.unwrap(),
                                                       self.progress,
                                                       "Validating...",
                                                       self.progress_message.as_str());
        if self.data.checksum.is_some() {
            view = view.push(
                Text::new(format!("Your checksum: {}", self.data.checksum.clone().unwrap())))
                .push(Button::new(&mut self.button_states[0],
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
