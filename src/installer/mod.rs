use iced::{Column, Text, Element, Button, button, TextInput, text_input, Subscription, ProgressBar, progress_bar, Background, Color, Command};
use super::common::{Message, Game, Installation, InstallationAttribute, format_ergc};
use super::reg;
use super::components::{InstallationEvent};
use super::checksums::{write_checksums_file};
use std::convert::identity;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs::{File, OpenOptions, create_dir_all, remove_dir_all};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use iced::progress_bar::Style;
use crate::checksums::{generate_files_list, ChecksumGenerator, calculate_hash};
use crate::common::InstallationProgress;
use handlebars::{Handlebars, RenderError};
use winreg::{RegValue, RegKey};
use winreg::enums::*;

#[derive(Debug, Clone)]
pub enum InstallerEvent {
    Next,
    RegistrationDone,
    // ExtractionProgressed((usize, ExtractionProgress)),
    // ChecksumGenerationProgressed((usize, ChecksumProgress)),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerStep {
    Inactive,
    Register,
    Download,
    Install,
    Validate,
    UserData,
    Done,
    Error
}

// #[derive(Debug, Clone, Copy)]
// pub enum InstallationProgress {
//     Started,
//     Advanced(f32),
//     Complete,
//     Failed
// }



impl InstallerStep {

    pub fn installation_steps() -> Vec<InstallerStep> {
        return vec![
            InstallerStep::Inactive, InstallerStep::Install,
            InstallerStep::Validate, InstallerStep::UserData, 
            InstallerStep::Register, InstallerStep::Done
        ]
    }

    pub fn validation_steps() -> Vec<InstallerStep> {
        vec![InstallerStep::Inactive, InstallerStep::Validate, InstallerStep::Register, InstallerStep::Done]
    }
}

#[derive(Debug, Clone)]
pub struct Installer {
    pub current_step: InstallerStep,
    // pub data: Installation,
    button_states: [button::State; 1],
    data_path_input_state: text_input::State,
    path_input_state: text_input::State,
    ergc_input_state: text_input::State,
    res_input_state: text_input::State,
    progress: f32,
    progress_message: String,
    steps: Vec<InstallerStep>,
    processing_state: ProcessingState
}

struct RegRenderData {
    pub install_path: String,
    pub install_path_shorthand: String,
    pub egrc: Option<String>
}


struct PbStyle {
    error: bool
}

#[derive(Debug, Clone)]
enum ProcessingState {
    Validation(String, Vec<String>, Vec<InstallationProgress>, String),
    Installation(Game, String, String, f32, String),
    UserDataInstallation(Game, String, f32, String),
    Failure(String),
    Idle
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

    pub fn new(steps: Vec<InstallerStep>) -> Installer {
        let mut installer = Installer {
            current_step: steps[0],
            // data: Installation::defaults(game),
            button_states: [button::State::default()],
            data_path_input_state: text_input::State::default(),
            path_input_state: text_input::State::default(),
            ergc_input_state: text_input::State::default(),
            res_input_state: text_input::State::default(),
            progress: 0.0,
            progress_message: String::from("NONE"),
            steps,
            processing_state: ProcessingState::Idle
        };
        installer

    }

    pub fn update(&mut self, installation: &Installation, event: InstallerEvent) -> Command<Message> {
        //let game = self.data.game;
        match event {
            InstallerEvent::Next => {
                self.proceed(installation);
                match self.current_step {
                    InstallerStep::Register => {
                        let install_path = String::from(&installation.path);
                        let ergc = String::from(&installation.ergc);
                        let checksum = String::from(&installation.checksum);
                        let game = installation.game;

                        let future = async move {
                            if let Err(msg) = Self::register(&install_path, &ergc, &checksum, &game) {
                                println!("ERROR: {}", msg);
                            }
                            game
                        };
                        Command::perform(future, |g| Message::InstallationEvent(g, InstallationEvent::InstallerEvent(InstallerEvent::RegistrationDone)))
                    },
                    _ => Command::none()
                }
            },
            InstallerEvent::RegistrationDone => {
                self.proceed(installation);
                Command::none()
            },
            _ => Command::none()
        }
    }

    pub fn view<'a>(&'a mut self, installation: &'a Installation) -> Element<Message> {
        match self.current_step {
            //InstallerStep::Configuration => self.config_view(),
            InstallerStep::Install => self.install_view(installation),
            InstallerStep::Validate => self.validate_view(installation),
            InstallerStep::UserData => self.validate_view(installation),
            InstallerStep::Register => self.registration_view(installation),
            InstallerStep::Done => self.completion_view(installation),
            InstallerStep::Error => self.error_view(installation),
            _ => self.default_view(installation)
        }
    }

    pub fn proceed(&mut self, installation: &Installation) -> Command<Message> {

        let i = self.steps.iter().position(|s| s == &self.current_step)
            .ok_or("").expect("Unexpected error (invalid installer state)");
        self.current_step = if i < self.steps.len() {
            self.steps[i+1]
        } else {
            self.current_step
        };
        
        self.processing_state = ProcessingState::Idle;

        match self.current_step {
            InstallerStep::Inactive => todo!(),
            InstallerStep::Register => {
                

                let install_path = String::from(&installation.path);
                let ergc = String::from(&installation.ergc);
                let checksum = String::from(&installation.checksum);
                let game = installation.game;

                let future = async move {
                    if let Err(msg) = Self::register(&install_path, &ergc, &checksum, &game) {
                        println!("ERROR: {}", msg);
                    }
                    game
                };
                Command::perform(future, |g| Message::Progressed((0, InstallationProgress::Finished)))
            },
            InstallerStep::Download => todo!(),
            InstallerStep::Install => {
                let install_source = installation.install_source.as_ref().ok_or(()).expect("Error: Installation source needs to be set!").clone();
                self.processing_state = ProcessingState::Installation(installation.game, install_source, installation.path.clone(), 0.0, String::from(""));
                Command::none()
            },
            InstallerStep::Validate => {
                let files = generate_files_list(PathBuf::from(&installation.path));

                self.processing_state = ProcessingState::Validation(installation.path.clone(), files, vec![], installation.path.clone());
                Command::none()
            },
            InstallerStep::UserData => {
                let install_source = installation.install_source.as_ref().ok_or(()).expect("Error: installation source not set!").clone();
                let game = installation.game;
                // let userdata_path = installation.get_userdata_path()
                //     .expect("Could not retrieve userdata path!");
                self.processing_state = ProcessingState::UserDataInstallation(game, install_source, 0.0, String::from(""));
                Command::none()
            },
            InstallerStep::Done => {
                let game = installation.game.clone();
                let future = async move {
                    game
                };
                Command::perform(future, |g| Message::InstallationComplete(g))
            },
            InstallerStep::Error => todo!(),
        }
    }

    pub fn subscriptions(&self, installation: &Installation) -> Vec<iced::Subscription<(usize, InstallationProgress)>> {
        match &self.processing_state {
            ProcessingState::Validation(install_path, files, _, _) => self.validation_task(install_path, files),
            ProcessingState::Installation(game, install_source, install_path, _, _) => self.installation_task(game.to_string(), install_source.clone(), install_path.clone()),
            ProcessingState::UserDataInstallation(game, install_source, _, _) => 
                self.userdata_installation_task(game.to_string(), install_source.clone(), installation),
            ProcessingState::Idle|ProcessingState::Failure(_) => vec![],
        }
    }

    pub fn installation_task(&self, game_str: String, install_source: String, install_path: String) -> Vec<iced::Subscription<(usize, InstallationProgress)>>  {
        let extraction_queue = (0..30)
            .map(|n| format!("{}/{}_{}.tar.gz", install_source, game_str, n))
            .filter(|p| Path::new(p).exists())
            .collect::<Vec<String>>();


        //let install_dir = install_source;
        println!("installing...");
        //remove_dir_all(&install_dir);
        create_dir_all(&install_path).expect(&format!("Error: Could not create directory {}", install_path));
        let mut extraction = super::extract::Extraction {
            id: 0,
            from: VecDeque::from(extraction_queue),
            to: install_path
        };

        vec![iced::Subscription::from_recipe(extraction)]
    }

    pub fn userdata_installation_task(&self, game_str: String, install_source: String, installation: &Installation) -> Vec<iced::Subscription<(usize, InstallationProgress)>> {
        let checksum = &installation.checksum;
        if checksum.is_empty() {
            return vec![];
        }

        let userdata_path = installation.get_userdata_path().expect("ERROR: Could not retrieve userdata path");

        let extraction_queue = vec![
            format!("{}/userdata.{}.tar.gz", install_source, game_str.to_lowercase())
        ];
        create_dir_all(&userdata_path);
        let mut extraction = super::extract::Extraction {
            id: 0,
            from: VecDeque::from(extraction_queue),
            to: String::from(userdata_path)
        };

        vec![iced::Subscription::from_recipe( extraction)]
    }

    pub fn on_progress(&mut self, installation: &Installation, progress: InstallationProgress) -> Command<Message> {
        if let InstallationProgress::Errored(msg) = progress {
            self.processing_state = ProcessingState::Failure(format!("Extraction failed: {}", msg));
            return Command::none()
        }

        match &mut self.processing_state {
            ProcessingState::Validation(install_path, files, ref mut results,  _) => {
                match progress {
                    InstallationProgress::ChecksumResult(path, checksum) => {
                        println!("Checksum calculated for {}", path);
                        results.push(InstallationProgress::ChecksumResult(path, checksum));
                        if files.len() == results.len() {
                            //Command::perform(future, |g| Message::InstallationEvent(g, InstallationEvent::InstallerEvent(InstallerEvent::RegistrationDone)))
                            
                            let validation_results = results.iter().map(|res| match res {
                                InstallationProgress::ChecksumResult(path, cs) => Ok(Some((path.clone(), cs.clone()))),
                                InstallationProgress::Skipped => Ok(None),
                                _ => Err(format!("Invalid validation result: {:#?}", res))
                            }).collect::<Result<Vec<Option<(String, String)>>, String>>()
                            .expect("Error while collecting validation results");

                            let game = installation.game.clone();
                            let install_path_clone = install_path.clone();

                            let future = async move {
                                let mut results = validation_results.into_iter().filter_map(identity)
                                .collect::<Vec<(String, String)>>();
                                results.sort_by(|(path1, _), (path2, _)| path1.cmp(path2));
                                write_checksums_file(&install_path_clone, results)
                                    .expect(&format!("Error writing {}\\checksums.txt!", &install_path_clone));
                                (
                                    game,
                                    calculate_hash(PathBuf::from(&install_path_clone).join("checksums.txt"))
                                        .expect(&format!("Error: Could not calculate checksum for {}\\checksums.txt!", &install_path_clone))
                                )
                            };

                            self.proceed(installation);
                            Command::perform(future, |(g, cs)| Message::AttributeUpdate(g, InstallationAttribute::Checksum, cs))
                        } else {
                            Command::none()
                        }
                    },
                    _ => Command::none()
                }
            },
            ProcessingState::Installation(game, install_source, install_path, _, _) => {
                match progress {
                    InstallationProgress::Extracting(prog, msg) => {
                        self.processing_state = ProcessingState::Installation(game.clone(), install_source.clone(), install_path.clone(), prog, msg);
                        Command::none()
                    },
                    InstallationProgress::Finished => {
                        self.proceed(installation);
                        Command::none()
                    },
                    _ => Command::none()
                }
            },
            ProcessingState::UserDataInstallation(game, install_source, _, _) => {
                match progress {
                    InstallationProgress::Finished => {
                    
                        let handlebars = Handlebars::new();
                        let options_file = PathBuf::from(
                            installation.get_userdata_path()
                                .expect("Could not retrieve userdata path!")
                        ).join("options.ini");
                        let mut options_tmpl = String::new();
                        File::open(&options_file)
                            .expect("Error opening options.ini for reading")
                            .read_to_string(&mut options_tmpl)
                            .expect("Error reading options.ini");

                        
                        let mut data = BTreeMap::new();
                        let (res_x, res_y) = installation.get_resolution();
                        data.insert("resolution", format!("{} {}", res_x, res_y));
                        
                        File::create(&options_file)
                            .unwrap()
                            .write(
                                handlebars.render_template(&options_tmpl, &data)
                                .expect("Error generating options.ini!")
                                .as_bytes()
                            )
                            .expect("Error writing options.ini!");
                        

                        
                        self.processing_state = ProcessingState::Idle;
                        self.proceed(installation)
                    },
                    InstallationProgress::Extracting(prog, msg) => {
                        self.processing_state = ProcessingState::UserDataInstallation(game.clone(), install_source.clone(), prog, msg);
                        Command::none()
                    },
                    _ => Command::none()
                }
            },
            ProcessingState::Idle => {
                match progress {
                    InstallationProgress::Finished => {
                        self.proceed(installation)
                    },
                    InstallationProgress::Errored(msg) => {
                        self.processing_state = ProcessingState::Failure(msg);
                        self.current_step = InstallerStep::Error;
                        Command::none()
                    }
                    _ => Command::none()
                }
            },
            ProcessingState::Failure(_) => todo!(),
        }
    }

    pub fn on_checksum_progressed(&mut self, update: (String, InstallationProgress)) {
        todo!()
    }

    pub fn validation_task(&self, install_path: &String, files: &Vec<String>) -> Vec<iced::Subscription<(usize, InstallationProgress)>> {
        files.iter()
            .enumerate()
            .map(|(id, path)| {
                iced::Subscription::from_recipe(ChecksumGenerator { id, install_path: install_path.clone(), path: String::from(path) })
            })
            .collect()
    }

    fn get_checksum(&self, installation: &Installation) -> Result<String, ()> {
        if installation.checksum.is_empty() {  Err(()) } else { Ok(installation.checksum.clone()) }
    }

    fn register(install_path: &str, ergc: &str, checksum: &str, game: &Game) -> Result<(), String> {
        let canon_path = PathBuf::from(install_path)
            .canonicalize()
            .unwrap().to_str()
            .unwrap()
            .replace("\\\\?\\", "");
        let mut reg_data = BTreeMap::new();
        reg_data.insert("install_path", canon_path.clone());
        reg_data.insert("install_path_shorthand", canon_path);
        reg_data.insert("ergc", String::from(ergc));
        reg_data.insert("checksum", String::from(checksum));
        let mut handlebars = Handlebars::new();


        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let reg_entries = match game {
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
            }).all(|result: Result<(), std::io::Error>| {
                if result.is_err() {
                    println!("ERROR: {}", result.unwrap_err());
                    return false
                }
                true
            })
        }).all(|result| result) {
            Ok(())
        } else {
            Err("An error occurred while setting up the registry".parse().unwrap())
        }

    }

    fn registration_view(&mut self, installation: &Installation) -> Element<Message> {

        //Message::InstallerNext(self.current_step.next());

        Column::new()
            .push(Text::new(format!("{:?}", self.current_step)))
            .push(Text::new("Writing registry entries..."))
            .into()
    }


    fn completion_view(&mut self, installation: &Installation) -> Element<Message> {

        //Message::InstallerNext(self.current_step.next());

        Column::new()
            .push(Text::new(format!("{:?}", self.current_step)))
            .push(Text::new("All done!"))
            .push(Button::new(&mut self.button_states[0],
                                     Text::new("Ok"))
            .on_press(Message::InstallationComplete(installation.game)))
            .into()
    }

    fn progress_view<'a>(installation: &Installation, progress: f32, title: &'a str, progress_message: String) -> Column<'a, Message> {
        println!("Progress: {}", progress);
        Column::new()
            .push(Text::new(format!("{:?}", installation.game)).size(20))
            .push(Text::new(format!("{}...", title)))
            .push(ProgressBar::new(0.0..=100.0, progress.abs().max(1.0))
                .style(PbStyle{error: match progress {
                    -100.0 => true,
                    _ => false
                }}))
            .push(Text::new(progress_message))
    }

    fn install_view<'a>(&'a mut self, installation: &'a Installation) -> Element<Message> {
        let (progress, message) = match &self.processing_state {
            ProcessingState::Installation(_, _, _, prog, msg) => Ok((*prog, msg.clone())),
            _ => Err(format!("{:#?}", self.processing_state))
        }.expect("Error: Unexpected installer state! ");
        let mut view = Self::progress_view(installation,
                                                       progress,
                                                       "Extracting...",
                                                       message);
        if self.progress == 100.0 {
            view = view.push(Button::new(&mut self.button_states[0],
                                         Text::new("Next"))
                .on_press(Message::InstallationEvent(
                    installation.game,
                    InstallationEvent::InstallerEvent(InstallerEvent::Next))));
        }

        view.into()
    }

    fn error_view<'a>(&'a mut self, installation: &'a Installation) -> Element<Message> {
        let message = match &self.processing_state {
            ProcessingState::Failure(msg) => msg.to_string(),
            _ => String::from("unexpected error")
        };

        Self::progress_view(installation, -100.0, "ERROR", message)
            .push(Button::new(&mut self.button_states[0], Text::new("Okay"))
                .on_press(Message::InstallationComplete(installation.game)))
            .into()
    }

    fn validate_view<'a>(&'a mut self, installation: &'a Installation) -> Element<Message> {
        
        let checksum = self.get_checksum(installation);
        let validation_complete = match self.current_step {
            InstallerStep::Validate => false,
            _ => true
        };
        // let (validation_progress, validation_message, userdata_progress) = match validation_complete {
        //     true => {
        //         (100.0, "", self.progress)
        //     },
        //     false => {
        //         (self.progress, self.progress_message.as_str().clone(), 0.0)
        //     }
        // };


        let progress_data = match &self.processing_state {
            ProcessingState::Validation(_, files, results, _) => {
                let progress = (results.len() as f32 * 100.0) / files.len() as f32;
                Some((progress, format!("file {} of {}", results.len() + 1, files.len()), 0.0 as f32))
            },
            ProcessingState::UserDataInstallation(_, _, prog, msg) => {
                Some((100.0 as f32, msg.clone(), prog.clone()))
            },
            _ => None
        };


        match progress_data {
            Some((validation_progress, validation_message, userdata_progress)) => {
                let mut view = Self::progress_view(installation, validation_progress, "Validating...", validation_message);
                match checksum {
                    Ok(cs) => {
                        view = view.push(
                            Text::new(format!("Your checksum: {}", cs.clone())));
                        if validation_complete {
                            view = view.push(Text::new("Setting up APPDATA..."))
                                .push(ProgressBar::new(0.0..=100.0, userdata_progress.abs().max(1.0))
                                    .style(PbStyle{error: match userdata_progress {
                                        -100.0 => true,
                                        _ => false
                                    }}))
                                .push(Text::new(self.to_owned().progress_message.as_str()));
                        }
                        // if userdata_progress == 100.0 {
                        //     view = view.push(Button::new(&mut self.button_states[0],
                        //                         Text::new("Next"))
                        //         .on_press(Message::InstallationEvent(
                        //             installation.game,
                        //             InstallationEvent::InstallerEvent(InstallerEvent::Next))));
                        // }
                    },
                    Err(cs) => {}
                }
                view
            }
            None => Column::new()
        }.into()
    }

    fn default_view<'a>(&'a mut self, installation: &'a Installation) -> Element<Message> {
        Column::new()
            .push(Text::new(format!("{:?}", installation.game)).size(20))
            .push(Text::new(format!("{:?}", self.current_step)))
            .push(Button::new(&mut self.button_states[0],
                              Text::new("Next"))
                .on_press(Message::InstallationEvent(
                    installation.game, 
                    InstallationEvent::InstallerEvent(InstallerEvent::Next))))
            .into()
    }

    // pub fn installer_from(installation: Installation) -> Installer {
    //     Installer::from(installation, InstallerStep::installation_steps())
    // }
    
    // pub fn validator_from(installation: Installation) -> Installer {
    //     Installer::from(installation, InstallerStep::validation_steps())
    // }
}