use super::installer::{InstallerStep};
use super::{extract, checksums};
use super::checksums::Progress as ValidationProgress;
use super::components::InstallationEvent;
use std::env;
use std::path::PathBuf;
use std::ptr::NonNull;
use md5::{Digest, Md5};
use md5::digest::Output;
use regex::Regex;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Cursor};
use std::str::from_utf8;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;
use base_emoji::try_from_str;
use crate::installer::InstallerEvent;
use crate::reg::get_reg_value;
use crate::checksums::md5sum;
use iced::{
    button, text_input, image
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Game {
    BFME2,
    ROTWK
}

impl Game {
    pub fn all() -> Vec<Game> {
        vec![Self::BFME2, Self::ROTWK]
    }
}

impl Hash for Game {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_string().hash(state)
    }
}

impl Display for Game{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Game::BFME2 => "BFME2",
            Game::ROTWK => "ROTWK"
        })
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    InstallationEvent(Game, InstallationEvent),
    AttributeUpdate(Game, InstallationAttribute, String),
    StartInstallation(Game),
    StartValidation(Game),
    InstallationComplete(Game),
    ValidationComplete(Game, String),
    Progressed((usize, InstallationProgress))
}


#[derive(Debug, Clone)]
pub enum InstallationProgress {
    Started,
    Finished,
    ChecksumResult(String, String),
    Extracting(f32, String),
    Progressed(u32),
    Errored(String),
    Skipped
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallationAttribute {
    Checksum, InstallPath, UserdataPath, ERGC, Resolution, InstallationSource
}


impl Hash for InstallationAttribute {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_string().hash(state)
    }
}


impl ToString for InstallationAttribute {
    fn to_string(&self) -> String {
        match self {
            Self::Checksum => "Checksum",
            Self::InstallPath => "Install Path",
            Self::UserdataPath => "Userdata Directory",
            Self::ERGC => "Activation Code",
            Self::Resolution => "Resolution",
            Self::InstallationSource => "Install Source"
        }.to_string()
    }
}

impl InstallationAttribute {
    pub fn all() -> Vec<InstallationAttribute> {
        vec![InstallationAttribute::Checksum, 
            InstallationAttribute::InstallPath, 
            InstallationAttribute::UserdataPath, 
            InstallationAttribute::ERGC,
            InstallationAttribute::Resolution,
            InstallationAttribute::InstallationSource]
    }
}

#[derive(Debug, Clone)]
pub struct Installation {
    pub game: Game,
    pub path: String,
    userdata_path: String,
    pub checksum: String,
    pub ergc: String,
    resolution: (u32, u32),
    pub install_source: Option<String>,
    pub is_complete: bool,
    pub in_progress: bool,
}

impl Installation {
    pub(crate) fn defaults(game: Game) -> Installation {
        return Installation {
            game,
            path: format!("C:\\Pgroam Files (x86)\\Electronic Arts\\{}", game),
            userdata_path: String::default(),
            checksum: String::default(),
            ergc: String::default(),
            resolution: (1024, 768),
            install_source: Some(env::current_dir()
                .expect("Could not retrieve current directory!")
                .canonicalize()
                .unwrap().to_str()
                .unwrap()
                .replace("\\\\?\\", ""),),
            is_complete: false,
            in_progress: false
        }
    }

    pub fn load(game: &Game) -> Result<Installation, String> {
        let game_slug = match game {
            Game::BFME2 => "The Battle for Middle-earth II",
            Game::ROTWK => "The Lord of the Rings, The Rise of the Witch-king"
        };
        let checksum_result = get_reg_value::<String>(
            HKEY_LOCAL_MACHINE,
            &*format!("SOFTWARE\\WOW6432Node\\Electronic Arts\\BFME2 Installer\\{}", game.to_string()),
            "checksum");
        let path_result = get_reg_value::<String>(
            HKEY_LOCAL_MACHINE,
            &*format!("SOFTWARE\\WOW6432Node\\Electronic Arts\\Electronic Arts\\{}", game_slug),
            "InstallPath"
        );
        let userdata_dir_result = get_reg_value::<String>(
            HKEY_LOCAL_MACHINE,
            &*format!("SOFTWARE\\WOW6432Node\\Electronic Arts\\Electronic Arts\\{}", game_slug),
            "UserDataLeafName"
        );
        //let data_path_result = Installation::defaults(*game).install_source;
        let ergc_result = get_reg_value::<String>(
            HKEY_LOCAL_MACHINE,
            &*format!("SOFTWARE\\WOW6432Node\\Electronic Arts\\Electronic Arts\\{}\\ergc", game_slug),
            ""
        );
        println!("Attempted to load data from registry. Found:\n '{:?}' '{:?}' '{:?}'' '{:?}'",
                 checksum_result, path_result, userdata_dir_result, ergc_result);
        match (checksum_result, path_result, userdata_dir_result, ergc_result) {
            (Ok(checksum), Ok(path), Ok(userdata_dir), Ok(ergc)) => {
                let userdata_path = dirs::home_dir().unwrap()
                    .join("AppData").join("Roaming")
                    .join(userdata_dir);
                let options_path = userdata_path.join("Options.ini");
                match File::open(&options_path) {
                    Ok(mut fh) => {
                        let mut options = String::new();
                        fh.read_to_string(&mut options);
                        match options.replace("\r", "").split("\n")
                            .filter(|s| s.starts_with("Resolution"))
                            .collect::<Vec<&str>>()
                            .first() {
                            Some(line) => {
                                let v = line.split(" ")
                                    .map(|s| s.parse::<u32>())
                                    .filter_map(|i| match i {
                                    Ok(val) => Some(val),
                                    Err(_) => None
                                }).collect::<Vec<u32>>();
                                match v.len() {
                                    2 => {
                                        let resolution = (v[0], v[1]);
                                        Ok(Installation {
                                            game: *game,
                                            checksum,
                                            path,
                                            userdata_path: userdata_path.to_str().unwrap().to_owned(),
                                            ergc,
                                            resolution,
                                            install_source: None,
                                            is_complete: true,
                                            in_progress: false
                                        })
                                    },
                                    _ => {
                                        Err(String::from("Error parsing options.ini"))
                                    }
                                }
                            },
                            None => Err(String::from("Could not find Resolution in options.ini!"))
                        }
                    },
                    _ => {
                        Err(String::from(format!("Couldn't open '{:?}'", options_path)))
                    }
                }
            }
            e => {
                Err(String::from(format!("Not matching: Got ({})", [e.0, e.1, e.2, e.3]
                    .map(|v| match v {
                        Ok(inner) => ("Ok", inner),
                        Err(inner) => ("Err", inner.to_string()),
                    }).iter().fold(String::default(), |v, w| v + &*format!("{}({}), ", w.0, w.1)))))
            }
        }

    }

    pub fn update_from(&mut self, other: Installation) {
        
        self.game = other.game.clone();
        self.path = other.path.clone();
        self.install_source = other.install_source.clone();
        self.userdata_path = other.userdata_path.clone();
        self.checksum = other.checksum.clone();
        self.ergc = other.ergc.clone();
        self.resolution = other.resolution.clone();
        self.is_complete = other.is_complete.clone();
    }

    pub fn get_userdata_path(&self) -> Option<String> {
        match self.userdata_path.is_empty() {
            true => {
                match self.checksum.is_empty() {
                    true => None,
                    false => {
                        Some(dirs::home_dir().unwrap()
                            .join("AppData").join("Roaming")
                            .join(format!("{}_{}",
                                          &self.game.to_string().to_lowercase(),
                                          self.checksum)).to_str().unwrap().to_owned())
                    }
                }

            }
            false => Some(String::from(&self.userdata_path))
        }
    }

    pub fn set_resolution(&mut self, res: String) -> Result<(), String> {
        let err: Result<(), String> = Err(format!("Could not parse resolution string: {}", res));
        match res.split_once("x") {
            Some((x, y)) => {
                match (x.parse::<u32>(), y.parse::<u32>()) {
                    (Ok(x), Ok(y)) => {
                        self.resolution = (x, y);
                        Ok(())
                    },
                    result => {
                        println!("Invalid resolution {} (must be in format: <number>x<number>", res);
                        Ok(())
                    }
                }
            },
            _ => {
                println!("Invalid resolution {} (must be in format: <number>x<number>", res);
                Ok(())
            }
        }
    }

    pub fn get_resolution(&self) -> (u32, u32) {
        self.resolution.clone()
    }

    pub fn get_resolution_string(&self) -> String {
        format!("{}x{}", self.resolution.0, self.resolution.1)
    }
    
    pub fn get_attribute<'a>(&'a self, attr: &'a InstallationAttribute) -> String {
        let res = self.get_resolution_string();
        let ergc = format_ergc(&self.ergc);
        match attr {
            InstallationAttribute::Checksum => self.checksum.clone(),
            InstallationAttribute::InstallPath => self.path.clone(),
            InstallationAttribute::UserdataPath => self.get_userdata_path().unwrap_or(String::from("<will be generated>")),
            //.expect(&format!("Error retrieving userdata path for {}", self.game)),
            InstallationAttribute::ERGC => ergc,
            InstallationAttribute::Resolution => res,
            InstallationAttribute::InstallationSource => self.install_source.as_ref().unwrap_or(&String::from("")).clone()
        }
    }

    pub fn set_attribute(&mut self, attr: &InstallationAttribute, value: String) -> Result<(), String> {
        match attr {
            InstallationAttribute::Checksum => {
                self.checksum = value;
            },
            InstallationAttribute::InstallPath => {
                self.path = value;
            },
            InstallationAttribute::UserdataPath => {
                // self.userdata_path = value;
                // self.get_userdata_path().unwrap_or(String::from("<will be generated>"))
            },
            //.expect(&format!("Error retrieving userdata path for {}", self.game)),
            InstallationAttribute::ERGC => {
                self.ergc = value.replace("-", "");
                println!("ERGC: {}", self.ergc);
            },
            InstallationAttribute::Resolution => {
                self.set_resolution(value)?
            }
            InstallationAttribute::InstallationSource => 
            {
                self.install_source = Some(value);
            },

        };
        
        Ok(())
    }

    pub fn get_full_checksum<'a>(&'a self, other: Option<&'a Installation>) -> Option<String> {
        match (self.game, other) {
            (Game::BFME2, _) => Some(self.checksum.clone()),
            (Game::ROTWK, None) => None,
            (Game::ROTWK, Some(other_inst)) => {
                if other_inst.checksum.is_empty() {
                    None
                } else {
                    let bfme2_checksum = other_inst.checksum.clone();
                    let full_checksum = md5sum::<Md5, _>(
                        &mut Cursor::new((bfme2_checksum + &self.checksum).as_bytes()))
                        .expect("ERROR: Could not create checksum over BFME2 and ROTWK individual checksums");
                    let md5_str = format!("{:x}", full_checksum);
                    Some(md5_str)
                }
            }
        }
        // if self.checksum == String::default() || (self.game == Game::ROTWK && other.checksum == String::default()) {
        //     println!("Not returning checksum.. game was {}, own cs: {}, other cs: {}", self.game, self.checksum, other.checksum);
        //     None
        // } else {
        //     match self.game {
        //         Game::BFME2 => Some(self.checksum.clone()),
        //         Game::ROTWK => {
        //             let bfme2_checksum = other.checksum.clone();
        //             let full_checksum = md5sum::<Md5, _>(
        //                 &mut Cursor::new((bfme2_checksum + &self.checksum).as_bytes()))
        //                 .expect("ERROR: Could not create checksum over BFME2 and ROTWK individual checksums");
        //             let md5_str = format!("{:x}", full_checksum);
        //             Some(md5_str)
        //         }
        //     }
        // }
    }

    pub fn is_installation_ready(&self) -> bool {
        let re = Regex::new(r"^([A-Z0-9]{4}-?){5}$").unwrap();

        (self.install_source.is_some()) 
        && PathBuf::from(&self.install_source.as_ref().unwrap()).is_dir() 
        && PathBuf::from(&self.install_source.as_ref().unwrap()).join(format!("{}_0.tar.gz", self.game)).exists()
        && re.is_match(&format_ergc(&self.ergc))
    }
}


#[derive(Debug, Clone)]
pub struct InstallationUIState {
    pub resolution_input: text_input::State,
    pub ergc_input: text_input::State,
    pub install_button: button::State,
    pub compat_image_checksum: image::viewer::State,
    pub compat_image_ergc: image::viewer::State
}

impl InstallationUIState {
    pub fn new() -> InstallationUIState {
        InstallationUIState{
            resolution_input: text_input::State::default(),
            ergc_input: text_input::State::default(),
            install_button: button::State::default(),
            compat_image_checksum: image::viewer::State::default(),
            compat_image_ergc: image::viewer::State::default()
        }
    }
}

pub fn format_ergc(ergc: &str) -> String {

    ergc
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
        .collect::<String>()
}

pub fn to_breakable(value: String) -> String {
    let result = value.chars()
        .enumerate()
        .flat_map(|(i, c)| [c,'\u{200B}']).collect::<String>();
    result
}

pub fn str_to_emoji_hash(value: String) -> Result<String, String> {
    match md5sum::<Md5, _>(&mut Cursor::new(value.as_bytes())) {
        Ok(checksum) => {
            for c in checksum {
                print!("{:02x} - ", c)
            };
            println!("");
            let emoji_string = base_emoji::to_string::<&[u8]>(checksum.iter().map(|b| b % 0xff).collect::<Vec<u8>>().as_slice());
            println!("{}", emoji_string);
            Ok(emoji_string)
        },
        Err(e) => Err(e)
    }

}