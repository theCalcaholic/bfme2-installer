use iced::{Column, Text, Settings, Application, executor, Command, Clipboard, Subscription, Color, Element, Container, Length, Button, button, Align};
use super::common::{Message, Game};

#[derive(Debug, Clone, Copy)]
pub enum InstallerStep {
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
pub struct InstallerData {
    pub game: Option<Game>,
    pub path: Option<String>,
    pub checksum: Option<String>,
    pub egrc: Option<String>
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
pub struct Installer {
    pub current_step: InstallerStep,
    button_states: [button::State; 1],
    pub data: InstallerData
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
