use base64::prelude::*;
use bincode::{Decode, Encode};
use eframe::{
    egui::{self, Frame, Margin},
    Storage,
};

use crate::{
    bytes_utils::{BytesDisplayVariant, BytesInput},
    PROFILES_KEY,
};

/// I drive
const DRIVE: &'static str = "drive";

#[derive(Encode, Decode, Clone)]
enum ProfileEntryKey {
    Key(BytesInput),
    Capture,
}

impl From<Vec<u8>> for ProfileEntryKey {
    fn from(value: Vec<u8>) -> Self {
        ProfileEntryKey::Key(BytesInput::new_from_bytes(value))
    }
}

impl ProfileEntryKey {
    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|line| {
            if line
                .radio(matches!(self, ProfileEntryKey::Key(_)), "Key")
                .clicked()
            {
                *self = ProfileEntryKey::Key(BytesInput::new());
            }
            if let ProfileEntryKey::Key(key) = self {
                key.draw(line);
            }
        });
        if ui
            .radio(matches!(self, ProfileEntryKey::Capture), "Capture")
            .clicked()
        {
            *self = ProfileEntryKey::Capture;
        }
    }
}

#[derive(Encode, Decode, Clone)]
struct ProfileEntry {
    key: ProfileEntryKey,
    alias: String,
    sub_items: Vec<ProfileEntry>,
    display: BytesDisplayVariant,
    collapsed: bool,
}

impl ProfileEntry {
    fn draw(&mut self, ui: &mut egui::Ui) {
        if self.collapsed {
            ui.horizontal(|line| {
                if line.button(egui_phosphor::variants::regular::PENCIL).clicked() {
                    self.collapsed = false;
                }
                line.label(&self.alias);
            });
        } else {
            ui.horizontal(|line| {
                if line
                    .button(egui_phosphor::variants::regular::ARROW_FAT_LINES_LEFT)
                    .clicked()
                {
                    self.collapsed = true;
                }
                line.label("Alias:");
                line.text_edit_singleline(&mut self.alias);
            });

            self.key.draw(ui);

            if matches!(self.key, ProfileEntryKey::Capture) {
                ui.collapsing("Captured value display", |collapsing| {
                    self.display.draw(collapsing);
                });
            }

            for sub_item in self.sub_items.iter_mut() {
                Frame::none()
                    .outer_margin(Margin {
                        left: 10.,
                        ..Default::default()
                    })
                    .show(ui, |frame| {
                        sub_item.draw(frame);
                    });
            }
        }
    }
}

fn default_profiles() -> Vec<(String, Vec<ProfileEntry>)> {
    let mut profiles = Vec::new();
    profiles.push((
        DRIVE.to_owned(),
        vec![
            ProfileEntry {
                collapsed: true,
                key: vec![64].into(),
                alias: "Data contract documents".to_string(),
                sub_items: vec![],
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                collapsed: true,
                key: vec![32].into(),
                alias: "Identities".to_string(),
                sub_items: vec![ProfileEntry {
                    collapsed: true,
                    key: ProfileEntryKey::Capture,
                    alias: "Identity {}".to_owned(),
                    sub_items: vec![],
                    display: BytesDisplayVariant::Hex,
                }],
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                collapsed: true,
                key: vec![24].into(),
                alias: "Unique public key hashes to identities".to_string(),
                sub_items: vec![],
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                collapsed: true,
                key: vec![8].into(),
                alias: "Non-unique public key Key hashes to identities".to_string(),
                sub_items: vec![],
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                collapsed: true,
                key: vec![48].into(),
                alias: "Pools".to_string(),
                sub_items: vec![],
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                collapsed: true,
                key: vec![40].into(),
                alias: "Pre funded specialized balances".to_string(),
                sub_items: vec![],
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                collapsed: true,
                key: vec![72].into(),
                alias: "Spent asset lock transactions".to_string(),
                sub_items: vec![],
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                collapsed: true,
                key: vec![104].into(),
                alias: "Misc".to_string(),
                sub_items: vec![],
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                collapsed: true,
                key: vec![80].into(),
                alias: "Withdrawal transactions".to_string(),
                sub_items: vec![],
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                collapsed: true,
                key: vec![96].into(),
                alias: "Balances".to_string(),
                sub_items: vec![],
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                collapsed: true,
                key: vec![16].into(),
                alias: "Token balances".to_string(),
                sub_items: vec![],
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                collapsed: true,
                key: vec![120].into(),
                alias: "Versions".to_string(),
                sub_items: vec![],
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                collapsed: true,
                key: vec![112].into(),
                alias: "Votes".to_string(),
                sub_items: vec![],
                display: BytesDisplayVariant::U8,
            },
        ],
    ));

    profiles
}

#[derive(Encode, Decode)]
pub(crate) struct ProfilesView {
    profiles: Vec<(String, Vec<ProfileEntry>)>,
    selected: String,
}

impl ProfilesView {
    pub(crate) fn persist(&self, storage: &mut dyn Storage) {
        if let Some(profiles_b64) = bincode::encode_to_vec(&self, bincode::config::standard())
            .ok()
            .map(|bytes| BASE64_STANDARD.encode(bytes))
        {
            storage.set_string(PROFILES_KEY, profiles_b64);
        }
    }

    pub(crate) fn restore(storage: Option<&dyn Storage>) -> Self {
        storage
            .and_then(|s| s.get_string(PROFILES_KEY))
            .and_then(|param| BASE64_STANDARD.decode(param).ok())
            .and_then(|encoded| {
                bincode::decode_from_slice(&encoded, bincode::config::standard())
                    .map(|(result, _)| result)
                    .inspect_err(|e| log::error!("{}", e))
                    .ok()
            })
            .unwrap_or_else(|| {
                log::error!("HUH");
                ProfilesView {
                    profiles: default_profiles(),
                    selected: DRIVE.to_string(),
                }
            })
    }

    pub(crate) fn draw(&mut self, ui: &mut egui::Ui) {
        ui.separator();

        let mut selected_profile = None;
        let mut copied_profiles = Vec::new();

        for (key, profile) in self.profiles.iter_mut() {
            let selected = self.selected.as_str() == key.as_str();
            ui.horizontal(|line| {
                if line.radio(selected, "").clicked() {
                    self.selected = key.to_owned();
                };
                if line.button("Copy").clicked() {
                    copied_profiles.push((format!("{key} copy"), profile.clone()));
                }
                line.text_edit_singleline(key);
                if selected {
                    self.selected = key.clone();
                }
            });
            if selected {
                selected_profile = Some(profile);
            }
        }

        ui.separator();

        if let Some(profile) = selected_profile {
            for item in profile.iter_mut() {
                item.draw(ui);
            }
        }

        self.profiles.extend_from_slice(&copied_profiles);
    }
}
