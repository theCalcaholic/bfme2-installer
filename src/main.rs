mod installer;
mod common;
mod extract;
mod checksums;
mod reg;
mod components;

use std::cell::Cell;
use std::io::{Cursor};
use std::collections::HashMap;
use common::InstallationProgress;
use installer::{Installer, InstallerStep};
use md5::Md5;
use common::{Message, Game, Installation, format_ergc, InstallationUIState};
use checksums::md5sum;

use iced::{
    image, Column, Text, Settings, Application, executor, Command,
    Clipboard, Element, Container, Length, Button, button, Subscription,
    Row, Space, text_input
};
use regex::Regex;
use blockies::{Classic, Ethereum};
use crate::common::to_breakable;
use crate::installer::InstallerEvent;
use crate::components::InstallationView;

// const ICONS: Font = Font::External {
//     name: "Icons",
//     bytes: include_bytes!("C://Windows/Fonts/seguiemj.ttf"),
//     //bytes: include_bytes!("resource/EmojiSymbols-Regular.woff"),
//     //bytes: include_bytes!("resource/EmojiOneColor.otf"),
// };


pub fn main() -> iced::Result {
    Bfme2Manager::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}


#[derive(Debug)]
struct Bfme2Manager {
    installations: (Installation, Installation),
    views: (InstallationView, InstallationView),
    // installations: HashMap<Game, (Installation, InstallationUIState, InstallationView)>,
    installer: Option<(Installer, Game)>,
    bfme2_install_button: button::State,
    rotwk_install_button: button::State,
    //inst_ui_states: Vec<(text_input::State, button::State, image::viewer::State, image::viewer::State)>,
}

impl Bfme2Manager {
    fn render_installations(&mut self) -> Element<Message> {
        let installer_views = match self.installer {
            Some((ref mut installer, game)) => {
                match game {
                    Game::BFME2 => (Some(installer.view(&self.installations.0)), None),
                    Game::ROTWK => (None, Some(installer.view(&self.installations.1)))
                }
            },
            None => (None, None)
        };
        
        Column::new().height(Length::Fill)
            .push(Text::new("Installed Games").size(40))
            .push(Space::with_height(Length::Units(20)))
            .push(Row::new().spacing(20)
                .push(Column::new().spacing(10).width(Length::FillPortion(1))
                    .push(self.views.0.render(&self.installations.0, installer_views.0, None)))
                .push(Column::new().spacing(10).width(Length::FillPortion(1))
                    .push(self.views.1.render(&self.installations.1, installer_views.1, Some(&self.installations.0)))))
            .into()
    }
}

impl Application for Bfme2Manager {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let installations = (
            Installation::load(&Game::BFME2).unwrap_or_else(|_| Installation::defaults(Game::BFME2)),
            Installation::load(&Game::ROTWK).unwrap_or_else(|_| Installation::defaults(Game::ROTWK)),
        );
        println!("Installations: {:#?}", installations);
        let (inst1, inst2) = installations.clone();
        (
            Bfme2Manager {
                installations: (installations.0, installations.1),
                views: (InstallationView::new(inst1.game), InstallationView::new(inst2.game)),
                //bfme2_view: InstallationView::new(),
                installer: None,
                bfme2_install_button: button::State::default(),
                rotwk_install_button: button::State::default(),
                //inst_ui_states
            },
            Command::none()
        )
    }

    fn title(&self) -> String {
        String::from("BFME2 LAN Manager")
    }

    fn update(&mut self, message: Self::Message, _clipboard: &mut Clipboard) -> Command<Self::Message> {
        match message {
            Message::StartInstallation(game)|Message::StartValidation(game) => {
                self.views.0.loose_focus();
                self.views.1.loose_focus();
                if self.installations.0.in_progress || self.installations.1.in_progress {
                    println!("There is already an installation in progress!");
                    return Command::none();
                }
                let steps = match message {
                    Message::StartInstallation(_) => InstallerStep::installation_steps(),
                    Message::StartValidation(_) => InstallerStep::validation_steps(),
                    _ => vec![InstallerStep::Inactive]
                };
                println!("steps: {:#?}", steps);
                let installation = match game {
                    Game::BFME2 => &mut self.installations.0,
                    Game::ROTWK => &mut self.installations.1
                };
                let mut installer = Installer::new(steps);
                installer.proceed(&installation);
                self.installer = Some((installer, installation.game));
                installation.in_progress = true;
                // view.set_installer(&mut installer);
                Command::none()
            },
            Message::Progressed((id, progress)) => {
                match self.installer {
                    Some((ref mut installer, game)) => {
                        match game {
                            Game::BFME2 => installer.on_progress(&self.installations.0, progress),
                            Game::ROTWK => installer.on_progress(&self.installations.1, progress)
                        }
                        
                    },
                    None => Command::none()
                }
            },
            Message::InstallationEvent(Game::BFME2, event) => {
                self.views.0.update(&self.installations.0, event)
            }
            Message::InstallationEvent(Game::ROTWK, event) => {
                self.views.1.update(&self.installations.1, event)
            },
            Message::AttributeUpdate(game, attr, value) => {
                match game {
                    Game::BFME2 => {self.installations.0.set_attribute(&attr, value)},
                    Game::ROTWK => {self.installations.1.set_attribute(&attr, value)}
                }.expect("Error while updating installation attribute");
                Command::none()

            }
            // Message::InstallerEvent(event) => {
            //     self.installer.as_mut().unwrap().update(event)
            // }
            Message::InstallationComplete(game) => {
                match game {
                    Game::BFME2 => {
                        self.installations.0.is_complete = true;
                        self.installations.0.in_progress = false;
                    },
                    Game::ROTWK => {
                        self.installations.1.is_complete = true;
                        self.installations.1.in_progress = false;
                    },
                }
                //self.installations.insert(game, (data, InstallationUIState::new(), InstallationView::new()));
                //self.inst_ui_states.push((text_input::State::default(), button::State::default(), image::viewer::State::new(), image::viewer::State::new()));
                self.installer = None;
                Command::none()
            }
            // Message::AttributeClicked(game, id) => {
            //     match game {
            //         Game::BFME2 => self.views.0.edit_attribute(id),
            //         Game::ROTWK => self.views.1.edit_attribute(id)
            //     };
            //     Command::none()
            // }
            m => {
                println!("msg: {:#?}", m);
                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {

        let subscriptions = match &self.installer {
            Some((installer, game)) => installer.subscriptions(match game {
                Game::BFME2 => &self.installations.0,
                Game::ROTWK => &self.installations.1
            }),
            None => vec![]
        };

        // let mut subscriptions = match &mut self.installations.0.2 {
        //     Some(installer) => installer.subscriptions(&self.installations.0.0),
        //     None => vec![]
        // };
        // subscriptions.extend(match self.installations.1.2 {
        //     Some(installer) => installer.subscriptions(&self.installations.1.0),
        //     None => vec![]
        // });
        Subscription::batch(subscriptions).map(Message::Progressed)

        // match self.installer.as_ref() {
        //     Some(installer) => match installer.current_step {
        //         // InstallerStep::Install => {
        //         //     installer.commence_install()
        //         //         .map(|v| Message::InstallerEvent(InstallerEvent::ExtractionProgressed(v)))
        //         // },
        //         InstallerStep::Validate => {
        //             installer.commence_generate_checksums(self.installations)
        //                 .map(|v| Message::InstallerEvent(InstallerEvent::ChecksumGenerationProgressed(v)))
        //         },
        //         // InstallerStep::UserData => {
        //         //     installer.commence_install_userdata()
        //         //         .map(|v| Message::InstallerEvent(InstallerEvent::ExtractionProgressed(v)))
        //         // }
        //         _ => Subscription::none()
        //     },
        //     None => Subscription::none()
        // }
    }

    fn view(&mut self) -> Element<Message> {

        Container::new(self.render_installations())
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .padding(20)
            .into()
    }
}
