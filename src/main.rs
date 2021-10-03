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

#[derive(Debug, Clone, Copy)]
enum Game {
    BFME2,
    ROTWK
}

#[derive(Debug, Clone, Copy)]
enum InstallerStep {
    Inactive,
    Configuration,
    Register,
    Download,
    Install,
    Validate
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
struct InstallerData {
    game: Option<Game>,
    path: Option<String>,
    checksum: Option<String>,
    egrc: Option<String>
}

impl InstallerData {
    fn defaults() -> InstallerData {
        return InstallerData {
            game: None,
            path: None,
            checksum: None,
            egrc: None
        }
    }
}

#[derive(Debug, Clone)]
struct Installer {
    current_step: InstallerStep,
    button_states: [button::State; 1],
    data: InstallerData
}

impl Installer {

    pub fn new() -> Installer {
        Installer {
            current_step: InstallerStep::Inactive,
            button_states: [button::State::default()],
            data: InstallerData::defaults()
        }
    }
    pub fn view(&mut self) -> Element<Message> {
        Column::new()
            .push(Text::new(format!("Installing {:?}", self.data.game.unwrap())).size(20))
            .push(Text::new(format!("{:?}", self.current_step)))
            .push(Button::new(&mut self.button_states[0],
                              Text::new("Next"))
                .on_press(Message::InstallerNext(self.current_step.next())))
            .into()
    }

    pub fn proceed(&mut self, step: InstallerStep) {

        self.current_step = step;
    }
}

#[derive(Debug)]
struct Installation {
    game: Game,
    path: String,
    checksum: String,
    egrc: String
}

#[derive(Debug, Clone, Copy)]
enum Screen {
    Dashboard,
    Installer(Game)
}

#[derive(Debug)]
struct Bfme2Manager {
    installations: Vec<Installation>,
    active_screen: Screen,
    install_button: button::State,
    installer: Installer
}

#[derive(Debug, Clone)]
enum Message {
    StartInstallation(Game),
    InstallerNext(InstallerStep)
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
                active_screen: Screen::Dashboard,
                install_button: button::State::default(),
                installer: Installer::new()
            },
            Command::none()
        )
    }

    fn title(&self) -> String {
        String::from("BFME2 LAN Manager")
    }

    fn update(&mut self, message: Self::Message, clipboard: &mut Clipboard) -> Command<Self::Message> {
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
