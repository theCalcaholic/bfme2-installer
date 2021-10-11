use super::installer::{InstallerStep, InstallationProgress};
use super::{extract, checksums};
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Read;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;
use crate::installer::InstallerEvent;
use crate::reg::get_reg_value;

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
    InstallerEvent(InstallerEvent),
    StartInstallation(Game),
    InstallationComplete(Game, Installation)
}

#[derive(Debug, Clone)]
pub struct Installation {
    pub game: Game,
    pub path: String,
    pub data_path: String,
    userdata_path: String,
    pub checksum: String,
    pub ergc: String,
    pub resolution: (u32, u32)
}

impl Installation {
    pub(crate) fn defaults(game: Game) -> Installation {
        return Installation {
            game,
            path: String::default(),
            userdata_path: String::default(),
            data_path: env::current_dir()
                .expect("Could not retrieve current directory!")
                .canonicalize()
                .unwrap().to_str()
                .unwrap()
                .replace("\\\\?\\", ""),
            checksum: String::default(),
            ergc: String::default(),
            resolution: (1024, 768)
        }
    }

    pub(crate) fn load(game: &Game) -> Option<Installation> {
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
        let data_path_result = Installation::defaults(*game).data_path;
        let ergc_result = get_reg_value::<String>(
            HKEY_LOCAL_MACHINE,
            &*format!("SOFTWARE\\WOW6432Node\\Electronic Arts\\Electronic Arts\\{}\\ergc", game_slug),
            ""
        );
        println!("Attempted to load data from registry. Found:\n '{:?}' '{:?}' '{:?}' '{}' '{:?}'",
                 checksum_result, path_result, userdata_dir_result, data_path_result, ergc_result);
        match (checksum_result, path_result, userdata_dir_result, data_path_result, ergc_result) {
            (Ok(checksum), Ok(path), Ok(userdata_dir), data_path, Ok(ergc)) => {
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
                                        Some(Installation {
                                            game: *game,
                                            checksum,
                                            path,
                                            userdata_path: userdata_path.to_str().unwrap().to_owned(),
                                            data_path,
                                            ergc,
                                            resolution
                                        })
                                    },
                                    _ => {
                                        println!("Error parsing options.ini");
                                        None
                                    }
                                }
                            },
                            None => None
                        }
                    },
                    _ => {
                        println!("Couldn't open '{}'",
                                 options_path.display());
                        None
                    }
                }
            }
            e => {
                println!("Not matching: Got ({})", [e.0, e.1, e.2, Ok(e.3), e.4]
                    .map(|v| match v {
                        Ok(inner) => ("Ok", inner),
                        Err(inner) => ("Err", inner.to_string()),
                    }).iter().fold(String::default(), |v, w| v + &*format!("{}({}), ", w.0, w.1)));
                None
            }
        }

    }

    pub fn get_userdata_path(&self) -> Result<String, ()> {
        let userdata_path = match self.userdata_path.is_empty() {
             true => {
                match self.checksum.is_empty() {
                    true => Err(()),
                    false => {
                        Ok(dirs::home_dir().unwrap()
                            .join("AppData").join("Roaming")
                            .join(format!("{}_{}",
                                          &self.game.to_string().to_lowercase(),
                                          self.checksum)).to_str().unwrap().to_owned())
                    }
                }

            }
            false => Ok(String::from(&self.userdata_path))
        };
        match userdata_path {
            Ok(path) => {
                //self.userdata_path = (&path).parse().unwrap();
                Ok((path).parse().unwrap())
            },
            Err(_) => Err(())
        }
    }
}

pub fn format_ergc(ergc: String) -> String {

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
    println!("in: {}", value);
    println!("out: {}", result);
    result
}