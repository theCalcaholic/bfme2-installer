use super::installer::{InstallerStep, InstallationProgress};
use super::extract;

#[derive(Debug)]
pub struct Installation {
    game: Game,
    path: String,
    checksum: String,
    egrc: String
}

#[derive(Debug, Clone, Copy)]
pub enum Game {
    BFME2,
    ROTWK
}

#[derive(Debug, Clone)]
pub enum Message {
    StartInstallation(Game),
    InstallerNext(InstallerStep),
    InstallerPathUpdate(String),
    InstallerInstallUpdate((usize, InstallationProgress)),
    ExtractionProgressed((usize, extract::Progress))
}
