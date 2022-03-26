mod installer;
mod common;
mod extract;
mod checksums;
mod reg;

use std::borrow::BorrowMut;
use std::collections::HashMap;
use installer::{Installer, InstallerStep};
use common::{Message, Game, Installation, format_ergc, str_to_emoji_hash};

use iced::{Column, Text, Settings, Application, executor, Command, Clipboard, Element, Container, Length, Button, button, Subscription, Color, Row, Space, Font, TextInput, text_input};
use iced::window::Mode;
use iced_native::Renderer;
use regex::Regex;
use crate::common::to_breakable;
use crate::installer::InstallerEvent;

const ICONS: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("C://Windows/Fonts/seguiemj.ttf"),
    //bytes: include_bytes!("resource/EmojiSymbols-Regular.woff"),
    //bytes: include_bytes!("resource/EmojiOneColor.otf"),
};


pub fn main() -> iced::Result {
    Bfme2Manager::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}


#[derive(Debug)]
struct Bfme2Manager {
    installations: HashMap<Game, Installation>,
    buttons: Vec<button::State>,
    installer: Option<Installer>,
    bfme2_install_button: button::State,
    rotwk_install_button: button::State,
    textinputs: Vec<text_input::State>
}

impl Bfme2Manager {

    fn dashboard(&mut self) -> Element<Message> {
        let mut installations_row = Row::new().spacing(20);
        //let old_buttons = &self.buttons.clone();
        //self.buttons = Vec::new();
        for ((installation, button), textinput) in self.installations.values().zip(self.buttons.iter_mut()).zip(self.textinputs.iter_mut()) {
            // let mut button = old_buttons.get(i).expect("Unexpected error!").clone();
            // self.buttons.push(button);
            // installations_row = installations_row.push(
            //     Self::installation_view(&installation, &mut button));

            let Installation{
                checksum,
                path,
                ergc,
                resolution,
                game,
                ..
            } = installation.to_owned();
            
            let param_header_size = 24;
            let param_title_size = 20;
            let param_value_size = 16;
            let param_emoji_size = 28;

            // TODO: Use identicons (e.g. https://crates.io/crates/identicon-rs)
            let emoji_char_re = Regex::new(r"\P{M}\p{M}*+").unwrap();
            let emoji_variant_re = Regex::new(r"\p{M}").unwrap();
            let checksum_emoji_quadrupels = match str_to_emoji_hash(String::from(&checksum)) {
                Ok(emoji_checksum) => {
                    let emoji_chars = emoji_char_re.find_iter(&emoji_checksum).map(|m| String::from(m.as_str())).collect::<Vec<String>>();
                    vec![0, 4, 8, 12].iter()
                        .map(|i| emoji_chars[*i..i+4].join(""))
                        .map(|s| emoji_variant_re.replace_all(&s, "").into_owned())
                        .collect::<Vec<String>>()
                },
                Err(_) => vec![String::from("not available"), String::from(""), String::from(""), String::from("")]
            };
            //let emoji_quadrupels = vec![0, 4, 8, 12].iter().map(|i| emoji_checksum.chars().take(i+4).skip(*i).collect::<String>()).collect::<Vec<String>>();
            //let emoji_chars = re.split(&emoji_checksum);

            for quad in &checksum_emoji_quadrupels {
                println!("{}", &quad);
                println!("{}", quad.chars()
                    .map(|c| format!("{} ({:x})", c, c as i32)).collect::<Vec<String>>().join("-"))
            }

            let ergc_emoji_quadrupels = match str_to_emoji_hash(String::from(&ergc)) {
                Ok(emoji_ergc) => {
                    let emoji_chars = emoji_char_re.find_iter(&emoji_ergc).map(|m| String::from(m.as_str())).collect::<Vec<String>>();
                    vec![0, 4, 8, 12].iter()
                        .map(|i| emoji_chars[*i..i+4].join(""))
                        .map(|s| emoji_variant_re.replace_all(&s, "").into_owned())
                        .collect::<Vec<String>>()
                },
                Err(_) => vec![String::from("not available"), String::from(""), String::from(""), String::from("")]
            };

            installations_row = installations_row.push(
                {
                let mut re = Regex::new(r"\p{M}*").unwrap();
                Column::new().spacing(10)
                    .push(Text::new(game.to_string()).size(26))
                    .push(Row::new().spacing(4)
                        .push(Text::new("Checksum: ").size(param_title_size))
                        .push(Text::new(&checksum).size(param_value_size)))
                    .push(Row::new().spacing(4)
                        .push(Text::new("Install Path: ").size(param_title_size))
                        .push(Text::new(to_breakable(path)).size(param_value_size)))
                    .push(Row::new().spacing(4)
                        .push(Text::new("Userdata Directory: ").size(param_title_size))
                        .push(Text::new(to_breakable(installation.get_userdata_path()
                                                .expect("Error retrieving userdata path"))).size(param_value_size)))
                    .push(Row::new().spacing(4)
                        .push(Text::new("Activation Code: ").size(param_title_size))
                        .push(Text::new(format_ergc(ergc)).size(param_value_size)))
                    .push(Row::new().spacing(4)
                        .push(Text::new("Resolution: ").size(param_title_size))
                        .push(Text::new(format!("{}x{}", resolution.0, resolution.1)).size(param_value_size)))
                    .push(Button::new(button,
                                      Text::new(format!("Reinstall {}", game)))
                        .on_press(Message::StartInstallation(game))).width(Length::FillPortion(1))
                    .push(Row::new().spacing(4).push(Text::new("Compatibility").size(param_header_size)))
                    .push(Row::new().push(Text::new("Must be equal: ")))
                    // .push(
                    //     TextInput::new(textinput, "", &emoji_quadrupels[1], |data| Message::InstallerEvent(InstallerEvent::ResolutionUpdate(String::from("1680x1050"))))
                    //     //TextInput::new(&emoji_quadrupels[0]).color(Color::BLACK).font(ICONS).size(param_emoji_size))
                    // )
                    .push(Row::new().push(Text::new(&checksum_emoji_quadrupels[0]).color(Color::BLACK).font(ICONS).size(param_emoji_size)))
                    .push(Row::new().push(Text::new(&checksum_emoji_quadrupels[1]).color(Color::BLACK).font(ICONS).size(param_emoji_size)))
                    .push(Row::new().push(Text::new(&checksum_emoji_quadrupels[2]).color(Color::BLACK).font(ICONS).size(param_emoji_size)))
                    .push(Row::new().push(Text::new(&checksum_emoji_quadrupels[3]).color(Color::BLACK).font(ICONS).size(param_emoji_size)))
                    .push(Row::new().push(Text::new("Must be different: ")))
                    .push(Row::new().push(Text::new(&ergc_emoji_quadrupels[0]).color(Color::BLACK).font(ICONS).size(param_emoji_size)))
                    .push(Row::new().push(Text::new(&ergc_emoji_quadrupels[1]).color(Color::BLACK).font(ICONS).size(param_emoji_size)))
                    .push(Row::new().push(Text::new(&ergc_emoji_quadrupels[2]).color(Color::BLACK).font(ICONS).size(param_emoji_size)))
                    .push(Row::new().push(Text::new(&ergc_emoji_quadrupels[3]).color(Color::BLACK).font(ICONS).size(param_emoji_size)))
                });
        }

        let mut buttons_col = Column::new().spacing(40);

        if !self.installations.contains_key(&Game::BFME2) {
            let mut bfme2_button = Button::new(&mut self.bfme2_install_button,
                                               Text::new(format!("Install {}", Game::BFME2)));
            bfme2_button = bfme2_button.on_press(Message::StartInstallation(Game::BFME2));
            buttons_col = buttons_col.push(bfme2_button)
        }
        if !self.installations.contains_key(&Game::ROTWK) {
            let mut rotwk_button = Button::new(&mut self.rotwk_install_button,
                                               Text::new(format!("Install {}", Game::ROTWK)));
            rotwk_button = rotwk_button.on_press(Message::StartInstallation(Game::ROTWK));
            buttons_col = buttons_col.push(rotwk_button);
        }


        installations_row = installations_row.push(buttons_col);
        // let not_installed: Vec<Game> = Game::all();
        // if not_installed.len() > 0 {
        //     for (game, mut button) in not_installed.iter().zip(&not_installed.iter().map(|g| match g {
        //         Game::BFME2 => self.bfme2_install_button.borrow_mut(),
        //         Game::ROTWK => self.rotwk_install_button.borrow_mut()
        //     }).collect::<Vec<button::State>>()) {
        //         // let mut button = match game {
        //         //     Game::BFME2 => &mut self.bfme2_install_button,
        //         //     Game::ROTWK => &mut self.rotwk_install_button
        //         // };
        //     }
        //
        // }

        Column::new().height(Length::Fill)
            .push(Text::new("Installed Games").size(40))
            .push(Space::with_height(Length::Units(20)))
            .push(installations_row)
            .into()
    }
}

impl Application for Bfme2Manager {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut installations = HashMap::new();
        let mut buttons: Vec<button::State> = Vec::new();
        let mut textinputs: Vec<text_input::State> = Vec::new();

        Game::all().iter()
            .filter_map(Installation::load)
            .for_each(|inst| {
                installations.insert(inst.game, inst);
                buttons.push(button::State::default());
                textinputs.push(text_input::State::default());
            });

        (
            Bfme2Manager {
                installations,
                buttons,
                installer: None,
                bfme2_install_button: button::State::default(),
                rotwk_install_button: button::State::default(),
                textinputs
            },
            Command::none()
        )
    }

    fn title(&self) -> String {
        String::from("BFME2 LAN Manager")
    }

    fn update(&mut self, message: Self::Message, _clipboard: &mut Clipboard) -> Command<Self::Message> {
        match message {
            Message::StartInstallation(game) => {
                let mut installer = if self.installations.contains_key(&game) {
                    Installer::from(self.installations[&game].clone())
                } else {
                    Installer::new(game)
                };
                installer.proceed();
                self.installer = Some(installer);
                Command::none()
            },
            Message::InstallerEvent(event) => {
                self.installer.as_mut().unwrap().update(event)
            }
            Message::InstallationComplete(game, data) => {
                self.installations.insert(game, data);
                self.buttons.push(button::State::default());
                self.installer = None;
                Command::none()
            }
            _ => Command::none()
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        match self.installer.as_ref() {
            Some(installer) => match installer.current_step {
                InstallerStep::Install => {
                    installer.commence_install()
                        .map(|v| Message::InstallerEvent(InstallerEvent::ExtractionProgressed(v)))
                },
                InstallerStep::Validate => {
                    installer.commence_generate_checksums()
                        .map(|v| Message::InstallerEvent(InstallerEvent::ChecksumGenerationProgressed(v)))
                },
                InstallerStep::UserData => {
                    installer.commence_install_userdata()
                        .map(|v| Message::InstallerEvent(InstallerEvent::ExtractionProgressed(v)))
                }
                _ => Subscription::none()
            },
            None => Subscription::none()
        }
    }

    fn view(&mut self) -> Element<Message> {

        let active_widget = if self.installer.is_none() {
            self.dashboard()
        } else {
            self.installer.as_mut().unwrap().view()
        };

        Container::new(active_widget)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .padding(20)
            .into()
    }
}
