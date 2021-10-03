use iced::{Column, Text, Element, Button, button, TextInput, text_input};
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
    pub path: String,
    pub checksum: Option<String>,
    pub egrc: Option<String>
}

impl InstallerData {
    fn defaults() -> InstallerData {
        return InstallerData {
            game: None,
            path: String::from(""),
            checksum: None,
            egrc: None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Installer {
    pub current_step: InstallerStep,
    pub data: InstallerData,
    button_states: [button::State; 1],
    path_input_state: text_input::State
}

impl Installer {

    pub fn new() -> Installer {
        Installer {
            current_step: InstallerStep::Inactive,
            data: InstallerData::defaults(),
            button_states: [button::State::default()],
            path_input_state: text_input::State::default()
        }
    }
    pub fn view(&mut self) -> Element<Message> {
        match self.current_step {
            InstallerStep::Configuration => self.config_view(),
            _ => self.default_view()
        }
    }

    pub fn proceed(&mut self, step: InstallerStep) {

        self.current_step = step;
    }

    fn config_view(&mut self) -> Element<Message>{
        Column::new()
            .push(Text::new(format!("Installing {:?}", self.data.game.unwrap())).size(20))
            .push(Text::new("Configuration"))
            .push(TextInput::new(&mut self.path_input_state,
                                 "install path", &self.data.path,
                                 Message::InstallerPathUpdate))
            .push(Text::new("Patch Level:"))
            .push(Button::new(&mut self.button_states[0],
                              Text::new("Next"))
                .on_press(Message::InstallerNext(self.current_step.next())))
            .push(Text::new(format!("data:\n{:?}", self.data)))
            .into()
    }

    fn default_view(&mut self) -> Element<Message> {
        Column::new()
            .push(Text::new(format!("Installing {:?}", self.data.game.unwrap())).size(20))
            .push(Text::new(format!("{:?}", self.current_step)))
            .push(Button::new(&mut self.button_states[0],
                              Text::new("Next"))
                .on_press(Message::InstallerNext(self.current_step.next())))
            .into()
    }
}
