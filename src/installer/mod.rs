use iced::{Column, Text, Element, Button, button, TextInput, text_input, Subscription, ProgressBar, progress_bar, Background, Color, Command};
use super::common::{Message, Game, Installation};
use super::extract;
use super::checksums;
use super::reg;
use std::hash::{Hash, Hasher};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs::{File, OpenOptions, create_dir_all, remove_dir_all};
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
use std::env;
use regex::internal::Inst;
use crate::installer::InstallerStep::Install;

#[derive(Debug, Clone)]
pub enum InstallerEvent {
    Next,
    InstallPathUpdate(String),
    DataPathUpdate(String),
    ErgcUpdate(String),
    ResolutionUpdate(String),
    ExtractionProgressed((usize, extract::Progress)),
    ChecksumGenerationProgressed((usize, checksums::Progress))
}

#[derive(Debug, Clone, Copy)]
pub enum InstallerStep {
    Inactive,
    Configuration,
    Register,
    Download,
    Install,
    Validate,
    UserData
}

#[derive(Debug, Clone, Copy)]
pub enum InstallationProgress {
    Started,
    Advanced(f32),
    Complete,
    Failed
}

impl InstallerStep {

    fn next(self) -> InstallerStep {
        match self {
            //InstallerStep::Configuration => InstallerStep::Download,
            InstallerStep::Inactive => InstallerStep::Configuration,
            InstallerStep::Configuration => InstallerStep::Install,
            InstallerStep::Install => InstallerStep::Validate,
            InstallerStep::Validate => InstallerStep::UserData,
            InstallerStep::UserData => InstallerStep::Register,
            InstallerStep::Register => InstallerStep::Inactive,
            _ => self.clone()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Installer {
    pub current_step: InstallerStep,
    pub data: Installation,
    button_states: [button::State; 1],
    data_path_input_state: text_input::State,
    path_input_state: text_input::State,
    ergc_input_state: text_input::State,
    res_input_state: text_input::State,
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

    pub fn new(game: Game) -> Installer {
        Installer {
            current_step: InstallerStep::Inactive,
            data: Installation::defaults(game),
            button_states: [button::State::default()],
            data_path_input_state: text_input::State::default(),
            path_input_state: text_input::State::default(),
            ergc_input_state: text_input::State::default(),
            res_input_state: text_input::State::default(),
            progress: 0.0,
            progress_message: String::from("NONE")
        }
    }

    pub fn update(&mut self, event: InstallerEvent) -> Command<Message> {
        match event {
            InstallerEvent::Next => {
                self.proceed();
                Command::none()
            },
            InstallerEvent::InstallPathUpdate(path) => {
                self.data.path = path;
                Command::none()
            },
            InstallerEvent::DataPathUpdate(path) => {
                self.data.data_path = path;
                Command::none()
            },
            InstallerEvent::ErgcUpdate(ergc) => {
                self.update_ergc(ergc);
                Command::none()
            },
            InstallerEvent::ResolutionUpdate(res_str) => {
                let vals = res_str.split("x").map(|i| i.parse::<u32>().unwrap()).collect::<Vec<u32>>();
                self.data.resolution = (vals[0], vals[1]);
                Command::none()
            }
            InstallerEvent::ExtractionProgressed(update) => {
                self.on_extraction_progressed(update);
                Command::none()
            },
            InstallerEvent::ChecksumGenerationProgressed(update) => {
                self.on_checksum_progress(update);
                Command::none()
            }
        }
    }

    pub fn view(&mut self) -> Element<Message> {
        match self.current_step {
            InstallerStep::Configuration => self.config_view(),
            InstallerStep::Install => self.install_view(),
            InstallerStep::Validate => self.validate_view(),
            InstallerStep::UserData => self.validate_view(),
            InstallerStep::Register => self.registration_view(),
            _ => self.default_view()
        }
    }

    pub fn proceed(&mut self) {

        self.current_step = InstallerStep::next(self.current_step);
    }

    pub fn commence_install(&self) -> iced::Subscription<(usize, extract::Progress)> {
        let game_str = self.data.game.to_string();
        let data_path = &self.data.data_path;
        let extraction_queue = (0..30)
            .map(|n| format!("{}/{}_{}.tar.gz", data_path, game_str, n))
            .filter(|p| Path::new(p).exists())
            .collect::<Vec<String>>();


        let install_dir = String::from(&self.data.path);
        println!("installing...");
        //remove_dir_all(&install_dir);
        create_dir_all(&install_dir);
        let mut extraction = extract::Extraction {
            id: 0,
            from: VecDeque::from(extraction_queue),
            to: install_dir
        };
        iced::Subscription::from_recipe( extraction)
    }

    pub fn commence_install_userdata(&self) -> iced::Subscription<(usize, extract::Progress)> {
        let data_path = &self.data.data_path;
        let game = &self.data.game;
        let extraction_queue = vec![
            format!("{}/userdata.{}.tar.gz", data_path, game.to_string().to_lowercase())
        ];

        let userdata_path = self.data.get_userdata_path()
            .expect("Could not retrieve userdata path!");
        create_dir_all(&userdata_path);
        let mut extraction = extract::Extraction {
            id: 0,
            from: VecDeque::from(extraction_queue),
            to: String::from(userdata_path)
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
                self.progress_message = String::from("");
                match self.current_step {
                    InstallerStep::UserData => {
                        let mut handlebars = Handlebars::new();
                        let options_file = PathBuf::from(
                            &self.data.get_userdata_path()
                                .expect("Could not retrieve userdata path!")
                        ).join("options.ini");
                        let mut options_tmpl = String::new();
                        File::open(&options_file).expect("Error reading options.ini").read_to_string(&mut options_tmpl);
    
                        
                        let mut data = BTreeMap::new();
                        data.insert("resolution", format!("{} {}", self.data.resolution.0, self.data.resolution.1));
                        
                        File::create(&options_file)
                            .unwrap()
                            .write(
                                handlebars.render_template(&options_tmpl, &data)
                                .expect("Error generating options.ini!")
                                .as_bytes()
                            );
                    },
                    _ => {}
                };
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
                self.progress_message = String::from("");
            },
            (_, checksums::Progress::Finished) => {
                self.progress = 100.0;
                self.progress_message = String::from("");
                self.calculate_checksum();
            },
            (_, checksums::Progress::Errored) => {
                self.progress = -100.0;
                self.progress_message = String::from("");
            }
        }
    }

    pub fn commence_generate_checksums(&self) -> iced::Subscription<(usize, checksums::Progress)> {
        let mut checksum_generator = checksums::ChecksumGenerator{
            id: 0,
            path: String::from(&self.data.path)
        };

        iced::Subscription::from_recipe(checksum_generator)
    }

    fn get_checksum(&self) -> Result<&str, ()> {
        match self.data.checksum.is_empty() {
            true => {
                // {
                //     let mut myself = self.to_owned();
                //     myself.calculate_checksum();
                // }
                match self.data.checksum.is_empty() {
                    true => Err(()),
                    false => Ok(self.data.checksum.as_str())
                }
            }
            false => Ok(self.data.checksum.as_str())
        }
    }

    fn calculate_checksum(&mut self) -> Result<(), ()> {

        let game_path = PathBuf::from(&self.data.path);

        self.data.checksum = match checksums::calculate_hash(game_path.join("checksums.txt")) {
            Ok(result) => {
                result
            },
            Err(e) => {
                String::default()
            }
        };

        return Err(())
    }

    fn register(data: &Installation) -> Result<(), String> {
        let canon_path = PathBuf::from(data.path.as_str())
            .canonicalize()
            .unwrap().to_str()
            .unwrap()
            .replace("\\\\?\\", "");
        let mut reg_data = BTreeMap::new();
        reg_data.insert("install_path", canon_path.clone());
        reg_data.insert("install_path_shorthand", canon_path);
        reg_data.insert("ergc", data.ergc.clone());
        reg_data.insert("checksum", data.checksum.clone());
        let mut handlebars = Handlebars::new();


        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let reg_entries = match data.game {
            Game::BFME2 => reg::BFME2,
            Game::ROTWK => reg::ROTWK,
        };

        if reg_entries.keys.get("HKLM").expect("Unexpected Error!")
            .entries()
            .map(|(key, entries)| {
            entries.entries().map(|(value_name, value)| {
                match value {
                    reg::RegValue::Str(val) => {
                        println!("Write {} to {}", val, key);
                        match hklm.create_subkey(key) {
                            Ok((reg_writer, disp)) => reg_writer.set_value(value_name, val),
                            Err(e) => Err(e)
                        }
                    },
                    reg::RegValue::UInt(val) => {
                        println!("Write {} to {}", val, key);
                        match hklm.create_subkey(key) {
                            Ok((reg_writer, disp)) => reg_writer.set_value(value_name, val),
                            Err(e) => Err(e)
                        }
                    },
                    reg::RegValue::Template(tmpl) => {
                        let val = handlebars.render_template(tmpl, &reg_data).unwrap();
                        println!("Write {} to {}", val, key);
                        match hklm.create_subkey(key) {
                            Ok((reg_writer, disp)) => reg_writer.set_value(value_name, &val),
                            Err(e) => Err(e)
                        }
                    },
                }
            }).all(|result: Result<(), std::io::Error>| result.is_ok())
        }).all(|result| result) {
            Ok(())
        } else {
            Err("An error occurred".parse().unwrap())
        }

    }

    fn registration_view(&mut self) -> Element<Message> {
        Self::register(&self.data);

        //Message::InstallerNext(self.current_step.next());

        Column::new()
            .push(Text::new(format!("Installing {:?}", self.data.game)).size(20))
            .push(Text::new(format!("{:?}", self.current_step)))
            .push(Text::new("Writing registry entries..."))
            .into()
    }

    pub fn update_ergc(&mut self, ergc: String) {
        let ergc_val = ergc.to_uppercase().replace("-", "");
        // let diff = (ergc_val.len() / 4) - (self.data.ergc.len() / 4);
        self.data.ergc = ergc_val;
        self.ergc_input_state.move_cursor_to_end();
        // self.ergc_input_state.move_cursor_to(
        //     match self.ergc_input_state.cursor().state(text_input::Value(ergc)) {
        //         text_input::cursor::State::Index(index) => index + diff,
        //         text_input::cursor::State::Selection(start, end) => end + diff
        //     }
        // );
    }

    fn config_view(&mut self) -> Element<Message>{
        let ergc_display = &self.data.ergc
            .replace("-", "").chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if i != 0 && i % 4 == 0 {
                    Some('-')
                } else {
                    None
                }
                .into_iter()
                .chain(std::iter::once(c))
            })
            .collect::<String>();
        let mut view = Column::new()
            .push(Text::new(format!("Installing {:?}", self.data.game)).size(20))
            .push(Text::new("Configuration"))
            .push(TextInput::new(&mut self.res_input_state, "display resolution",
                                 &format!("{}x{}", self.data.resolution.0, self.data.resolution.1),
                                 |data| Message::InstallerEvent(InstallerEvent::ResolutionUpdate(data))))
            .push(TextInput::new(&mut self.data_path_input_state, "data path", 
                                 &self.data.data_path,
                                 |data| Message::InstallerEvent(InstallerEvent::DataPathUpdate(data))))
            .push(TextInput::new(&mut self.path_input_state,
                                 "install path", &self.data.path,
                                 |data| Message::InstallerEvent(InstallerEvent::InstallPathUpdate(data))))
            .push(TextInput::new(&mut self.ergc_input_state,
            "activation code", ergc_display,
                                 |data| Message::InstallerEvent(InstallerEvent::ErgcUpdate(data))))
            .push(Text::new("You can get a valid activation key from here: https://www.youtube.com/watch?v=eWg680bt_es"));
            if PathBuf::from(&self.data.data_path).is_dir()
                && PathBuf::from(&self.data.data_path).join(format!("{}_0.tar.gz", &self.data.game)).exists()
                && Regex::new(r"^([A-Z0-9]{4}-?){5}$").unwrap()
                .is_match(self.data.ergc.replace("-", "").to_uppercase().as_str()) {
                view = view.push(Button::new(&mut self.button_states[0],
                                      Text::new("Next"))
                    .on_press(Message::InstallerEvent(InstallerEvent::Next)))
            }
            view//.push(Text::new(format!("data:\n{:?}", self.data)))
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
        let mut view = Self::progress_view(self.data.game,
                                                       self.progress,
                                                       "Extracting...",
                                                       self.progress_message.as_str());
        if self.progress == 100.0 {
            view = view.push(Button::new(&mut self.button_states[0],
                                         Text::new("Next"))
                .on_press(Message::InstallerEvent(InstallerEvent::Next)));
        }

        view.into()
    }

    fn validate_view(&mut self) -> Element<Message> {
        
        let checksum = self.get_checksum();
        let (validation_progress, validation_message, userdata_progress) = match checksum {
            Ok(cs) => {
                (100.0, "", self.progress)
            },
            Err(_) => {
                (self.progress, self.progress_message.as_str().clone(), 0.0)
            }
        };
        let mut view = Self::progress_view(self.data.game.to_owned(),
                                                    validation_progress,
                                                    "Validating...",
                                                    validation_message);
                        
        {
            match checksum {
                Ok(cs) => {
                    view = view.push(
                        Text::new(format!("Your checksum: {}", cs.clone())));
                    if userdata_progress > 0.0 {
                        view = view.push(Text::new("Setting up APPDATA..."))
                            .push(ProgressBar::new(0.0..=100.0, userdata_progress.abs())
                                .style(PbStyle{error: match userdata_progress {
                                    -100.0 => true,
                                    _ => false
                                }}))
                            .push(Text::new(self.to_owned().progress_message.as_str()));
                    }
                    if userdata_progress == 100.0 {
                        view = view.push(Button::new(&mut self.button_states[0],
                                            Text::new("Next"))
                            .on_press(Message::InstallerEvent(InstallerEvent::Next)));
                    }
                },
                Err(cs) => {}
            }
            view.into()
        }
    }

    fn default_view(&mut self) -> Element<Message> {
        Column::new()
            .push(Text::new(format!("Installing {:?}", self.data.game)).size(20))
            .push(Text::new(format!("{:?}", self.current_step)))
            .push(Button::new(&mut self.button_states[0],
                              Text::new("Next"))
                .on_press(Message::InstallerEvent(InstallerEvent::Next)))
            .into()
    }
}

impl From<Installation> for Installer {
    fn from(data: Installation) -> Installer {

        Installer {
            data: data.clone(),
            ..Installer::new(data.game)
        }

    }
}