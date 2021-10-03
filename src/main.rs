mod installer;
mod common;

use installer::{Installer, InstallerStep};
use common::{Message, Game, Installation};

use std::ptr::null;
//use iced::{canvas::{self, Cache, Canvas, Cursor, Geometry, LineCap, Path, Stroke}, executor, time, Application, Color, Command, Container, Element, Length, Point, Rectangle, Settings, Subscription, Vector, Clipboard};
use iced::{Column, Text, Settings, Application, executor, Command, Clipboard, Subscription, Color, Element, Container, Length, Button, button, Align};
use iced::window::Mode;
use iced_native::{Renderer, Widget};

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
            Message::InstallerNext(mut step) => {
                match step {
                    InstallerStep::Inactive => {
                        self.installer.current_step = InstallerStep::Inactive;
                    }
                    _ => {
                        self.installer.proceed(step)
                    }
                }
                Command::none()
            }
            _ => Command::none()
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
