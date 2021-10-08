mod installer;
mod common;
mod extract;
mod checksums;
mod reg;

use installer::{Installer, InstallerStep};
use common::{Message, Game, Installation};

use iced::{Column, Text, Settings, Application, executor, Command, Clipboard, Element, Container, Length, Button, button, Subscription, Color};
use iced::window::Mode;

pub fn main() -> iced::Result {
    Bfme2Manager::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}


#[derive(Debug)]
struct Bfme2Manager {
    installations: Vec<Installation>,
    install_button: button::State,
    installer: Installer
}

impl Bfme2Manager {

    fn dashboard(&mut self) -> Element<Message> {
        Column::new()
            .push(Text::new("Installations").size(20))
            .push(Button::new(&mut self.install_button,
                              Text::new("(Re-)Install BFME"))
                .on_press(Message::StartInstallation(Game::BFME2)))
            .into()
    }}

impl Application for Bfme2Manager {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Bfme2Manager {
                installations: Vec::new(),
                install_button: button::State::default(),
                installer: Installer::new()
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
                self.installer = Installer::new();
                self.installer.data.game = Some(game);
                self.installer.current_step = InstallerStep::Configuration;
                Command::none()
            },
            Message::InstallerNext(step) => {
                match step {
                    InstallerStep::Inactive => {
                        self.installer.current_step = InstallerStep::Inactive;
                    }
                    _ => {
                        self.installer.proceed(step)
                    }
                }
                Command::none()
            },
            Message::InstallerConfigUpdate(path) => {
                self.installer.data.path = path;
                Command::none()
            },
            Message::ExtractionProgressed(update) => {
                self.installer.on_extraction_progressed(update);
                Command::none()
            },
            Message::ChecksumGenerationProgressed(update) => {
                self.installer.on_checksum_progress(update);
                Command::none()
            }

            _ => Command::none()
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        match self.installer.current_step {
            InstallerStep::Install => {
                self.installer.install().map(Message::ExtractionProgressed)
            },
            InstallerStep::Validate => {
                self.installer.generate_checksums().map(Message::ChecksumGenerationProgressed)
            },
            InstallerStep::UserData => {
                self.installer.install_userdata().map(Message::ExtractionProgressed)
            }
            _ => Subscription::none()
        }
    }

    fn view(&mut self) -> Element<Message> {

        let active_widget = match self.installer.current_step {
            InstallerStep::Inactive => self.dashboard(),
            _ => self.installer.view()
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
