mod installer;
mod common;
mod extract;
mod checksums;
mod reg;

use std::collections::HashMap;
use installer::{Installer, InstallerStep};
use common::{Message, Game, Installation};

use iced::{Column, Text, Settings, Application, executor, Command, Clipboard, Element, Container, Length, Button, button, Subscription, Color};
use iced::window::Mode;
use crate::installer::InstallerEvent;

pub fn main() -> iced::Result {
    Bfme2Manager::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}


#[derive(Debug)]
struct Bfme2Manager {
    installations: HashMap<Game, Installation>,
    bfme2_install_button: button::State,
    rotwk_install_button: button::State,
    installer: Option<Installer>
}

impl Bfme2Manager {

    fn dashboard(&mut self) -> Element<Message> {
        Column::new()
            .push(Text::new("Installations").size(20))
            .push(Button::new(&mut self.bfme2_install_button,
                              Text::new("(Re-)Install BFME"))
                .on_press(Message::StartInstallation(Game::BFME2)))
            .push(Button::new(&mut self.rotwk_install_button,
                                Text::new("(Re-)Install ROTWK"))
                .on_press(Message::StartInstallation(Game::ROTWK)))
            .into()
    }}

impl Application for Bfme2Manager {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Bfme2Manager {
                installations: {
                    let mut map = HashMap::new();
                    Game::all().iter()
                        .filter_map(Installation::load)
                        .for_each(|inst| { map.insert(inst.game, inst); ()});
                    map
                },
                bfme2_install_button: button::State::default(),
                rotwk_install_button: button::State::default(),
                installer: None
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
                },
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
