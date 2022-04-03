use std::collections::HashMap;
use std::path::PathBuf;
use iced::{text_input, button, image, Subscription};
use iced::{
    Command, Column, Element, Text, Row, Length, 
    VerticalAlignment, TextInput, Button, Container, Align,
    Color, Background, Vector, Space, HorizontalAlignment};
use regex::Regex;
use regex::internal::Inst;
use crate::common::InstallationProgress;

use super::common::{Installation, Message, Game, InstallationAttribute, to_breakable, format_ergc};
use super::installer::{InstallerEvent, Installer};
use super::checksums::{md5sum};
use md5::Md5;
use std::io::{Cursor};
use blockies::Ethereum;

#[derive(Debug, Clone)]
struct Layout {
    header_size: u16,
    title_size: u16,
    value_size: u16,
    text_height: u16
}

#[derive(Debug, Clone)]
pub struct InstallationView {
    pub game: Game,
    //installation: Installation,
    bfme2_checksum: Option<String>,
    resolution_input: text_input::State,
    ergc_input: text_input::State,
    compat_image_checksum: image::viewer::State,
    compat_image_ergc: image::viewer::State,
    layout: Layout,
    attributes: Vec<AttributeView>,
    editing: Option<InstallationAttribute>,
    compat_views: (CompatibilityView, CompatibilityView),
    //installer: Option<Installer>,
    install_button: button::State,
    validate_button: button::State
}


struct AttributeButtonStyle {
    editable: bool
}

impl button::StyleSheet for AttributeButtonStyle {
    fn active(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border_color: Color::new(0.3, 0.3, 0.3, 0.2),
            border_radius: if self.editable { 0.0 } else { 0.0 },
            border_width: if self.editable { 0.0 } else { 0.0 },
            shadow_offset: Vector::new(0.0, 0.0),
            text_color: Color::BLACK,
        }
    }

    fn hovered(&self) -> button::Style {
        match self.editable {
            true => button::Style {
                background: Some(Background::Color(Color::new(0.0, 0.0, 0.8, 0.2))),
                ..button::Style::default()
            },
            false => self.active()
        }
    }

}


impl InstallationView {
    pub fn new(game: Game) -> Self {
        let layout = Layout {
            header_size: 24,
            title_size: 20,
            value_size: 16,
            text_height: 36
        };
        Self{
            game,
            bfme2_checksum: None,
            resolution_input: text_input::State::default(),
            ergc_input: text_input::State::default(),
            compat_image_checksum: image::viewer::State::default(),
            compat_image_ergc: image::viewer::State::default(),
            layout: layout.clone(),
            attributes: InstallationAttribute::all().into_iter()
                .map(|attr| AttributeView::new(attr, game, layout.clone()))
                .collect::<Vec<AttributeView>>(),
            editing: None,
            compat_views: (
                CompatibilityView::new(InstallationAttribute::Checksum, game, layout.clone()),
                CompatibilityView::new(InstallationAttribute::ERGC, game, layout.clone())),
            install_button: button::State::default(),
            validate_button: button::State::default()
        }
    }

    pub fn update(&mut self, installation: &Installation, event: InstallationEvent) -> Command<Message> {
        match event {
            InstallationEvent::AttributeClicked(attr) => {
                if Self::can_edit(attr, installation) {
                    self.editing = Some(attr);
                } else {
                    self.editing = None;
                }
            }
            _ => {}
        }
        Command::none()
    }

    pub fn loose_focus(&mut self) {
        self.editing = None
    }

    pub fn render<'a>(&'a mut self, installation: &'a Installation, installer_view: Option<Element<'a, Message>>, other_installation: Option<&Installation>) -> Element<'a, Message> {
        let Installation{game, ergc, ..} = installation;
        
        let mut col = Column::new()
            .push(Text::new(game.to_string())
                .size(self.layout.header_size)
                .vertical_alignment(VerticalAlignment::Center)
                .height(Length::Units(self.layout.text_height+4)));
        for attr in &mut self.attributes {
            if attr.id == InstallationAttribute::InstallationSource && (installation.is_complete || installation.in_progress) {
                continue;
            }
            let is_editable = Self::can_edit(attr.id, installation);
            let is_edited = match self.editing { 
                Some(id) => id == attr.id && is_editable, 
                None => false 
            };
            col = col.push(
                attr.view(
                    installation.get_attribute(&attr.id).clone(), 
                    is_edited, 
                    is_editable))
        };

        if ! installation.is_complete && ! installation.in_progress {
            col = col.push(Row::new())
        }

        let ergc_checksum = if ergc.to_string() == String::default() {
            None
        } else {
            let ergc_md5 = md5sum::<Md5, _>(&mut Cursor::new(ergc.as_bytes()))
                .expect("ERROR: Could not create checksum over ERGC");
            Some(format!("{:x}", ergc_md5))
        };

        col = if installer_view.is_some() && installation.in_progress {

        col.push(Space::new(Length::Fill, Length::Units(60)))
            .push(installer_view.unwrap())

        } else {
            let mut install_button = Button::new(
                    &mut self.install_button, 
                    Text::new(match installation.is_complete {true => "Reinstall", false => "Install"})
                        .horizontal_alignment(HorizontalAlignment::Center))
                .width(Length::FillPortion(1));
            if ! installation.in_progress && installation.is_installation_ready() {
                install_button = install_button.on_press(Message::StartInstallation(self.game));
            }

            let mut validate_button = Button::new(
                    &mut self.validate_button, 
                    Text::new("Validate")
                        .horizontal_alignment(HorizontalAlignment::Center))
                .width(Length::FillPortion(1));
            if ! installation.in_progress && installation.is_complete {
                validate_button = validate_button.on_press(Message::StartValidation(self.game))
            }
            
            col = col.push(Row::new().spacing(10)
                .push(install_button)
                .push(validate_button));
            
            if installation.is_complete {
                let full_checksum = installation.get_full_checksum(other_installation);

                col = col.push(Space::new(Length::Fill, Length::Units(60)))
                    .push(Row::new()
                        .push(Text::new("Compatibility")
                            .size(self.layout.title_size)
                            .width(Length::Fill)
                            .vertical_alignment(VerticalAlignment::Center)
                            .horizontal_alignment(HorizontalAlignment::Center)
                            .height(Length::Units(self.layout.text_height))))
                    .push(Row::new()
                        .push(Column::new().width(Length::FillPortion(1)).align_items(Align::Center)
                            .push(Row::new()
                                .push(Text::new("Must be equal")
                                    .size(self.layout.value_size)
                                    .height(Length::Units(self.layout.text_height))
                                    .vertical_alignment(VerticalAlignment::Center)))
                            .push(Row::new().push(self.compat_views.0.view(full_checksum))))
                        .push(Column::new().width(Length::FillPortion(1)).align_items(Align::Center)
                            .push(Row::new()
                                .push(Text::new("Must be different")
                                    .size(self.layout.value_size)
                                    .height(Length::Units(self.layout.text_height))
                                    .vertical_alignment(VerticalAlignment::Center)))
                            .push(Row::new().push(self.compat_views.1.view(ergc_checksum)))))
            };
            col
        };
        col.into()
        
    }

    fn can_edit(attr: InstallationAttribute, installation: &Installation) -> bool {
        let mut editables = vec![InstallationAttribute::ERGC, InstallationAttribute::Resolution, InstallationAttribute::InstallationSource];
        if !installation.is_complete {
            editables.push(InstallationAttribute::InstallPath);
        }
        ! installation.in_progress && editables.contains(&attr)
    }

    pub fn edit_attribute(&mut self, id: InstallationAttribute) {
        self.editing = Some(id);
    }
    
}

#[derive(Debug, Clone)]
struct AttributeView {
    id: InstallationAttribute,
    game: Game,
    title: String,
    layout: Layout,
    input: text_input::State,
    button: button::State
}

impl AttributeView {
    fn new(id: InstallationAttribute, game: Game, layout: Layout) -> Self {
        Self {
            id,
            game,
            title: id.to_string(),
            layout,
            input: text_input::State::default(),
            button: button::State::default()
        }
    }

    fn view<'a>(&mut self, value: String, editing: bool, editable: bool) -> Element<Message> {
        let game = self.game;
        let id = self.id;

        if editing {
            self.input.focus();
        }

        Row::new()
            .spacing(4)
            .push(Text::new(self.title.clone())
                .size(self.layout.title_size)
                .height(Length::Units(self.layout.text_height))
                .vertical_alignment(VerticalAlignment::Center))
        .push::<Element<Message>>(match editing {
            true => Container::new(
                    TextInput::new(&mut self.input, &self.title, &value, 
                        move |data| Message::AttributeUpdate(game, id, data)))
                .height(Length::Units(self.layout.text_height))
                .width(Length::Fill)
                .center_y()
                .into(),
            false => Button::new(&mut self.button, Text::new(to_breakable(value))
                        .size(self.layout.value_size)
                        .height(Length::Units(self.layout.text_height))
                        .vertical_alignment(VerticalAlignment::Center))
                    .style(AttributeButtonStyle{editable})
                    .height(Length::Units(self.layout.text_height))
                    .width(Length::Fill)
                    .padding(0)
                    .on_press(Message::InstallationEvent(self.game, InstallationEvent::AttributeClicked(self.id))).into()
        }).into()
            
    }
}

impl PartialEq for AttributeView {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[derive(Debug, Clone)]
struct CompatibilityView {
    id: InstallationAttribute,
    game: Game,
    layout: Layout,
    image_state: image::viewer::State
}

impl CompatibilityView {

    fn new(id: InstallationAttribute, game: Game, layout: Layout) -> Self{
        Self {id, game, layout, image_state: image::viewer::State::default()}
    }

    fn view(&mut self, value: Option<String>) -> Element<Message>{
        let mut blockies = Ethereum::default();
        blockies.size = 8;
        blockies.scale = 16;
        let mut checksum_png = Vec::new();

        Row::new().push::<Element<Message>>( match value {
                Some(checksum) => {
                    blockies.create_icon(&mut checksum_png, checksum.as_bytes().into())
                        .expect("Error: Could not create identicon for checksum!");
                    image::Viewer::new(&mut self.image_state, image::Handle::from_memory(checksum_png))
                        .into()
                },
                None => Text::new("Not available")
                    .size(self.layout.value_size)
                    .height(Length::Units(self.layout.text_height))
                    .vertical_alignment(VerticalAlignment::Center)
                    .into()
            }).into()
    }
}

#[derive(Debug, Clone)]
pub enum InstallationEvent {
    AttributeUpdate(InstallationAttribute, String),
    AttributeClicked(InstallationAttribute),
    InstallerEvent(InstallerEvent),
    StartInstallation,
    Stub
}