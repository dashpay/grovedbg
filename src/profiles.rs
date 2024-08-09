use std::collections::BTreeMap;

use base64::prelude::*;
use bincode::{Decode, Encode};
use eframe::{egui, Storage};

use crate::{
    bytes_utils::{BytesDisplayVariant, BytesInput},
    PROFILES_KEY,
};

/// I drive
const DRIVE: &'static str = "drive";

#[derive(Encode, Decode)]
enum ProfileEntryKey {
    Root,
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
        if ui.radio(matches!(self, ProfileEntryKey::Root), "Root").clicked() {
            *self = ProfileEntryKey::Root;
        }
        if ui
            .radio(matches!(self, ProfileEntryKey::Key(_)), "Capture")
            .clicked()
        {
            *self = ProfileEntryKey::Key(BytesInput::new());
        }
        if ui
            .radio(matches!(self, ProfileEntryKey::Capture), "Capture")
            .clicked()
        {
            *self = ProfileEntryKey::Capture;
        }
    }
}

#[derive(Encode, Decode)]
struct ProfileEntry {
    key: ProfileEntryKey,
    alias: String,
    sub_items: Vec<ProfileEntry>,
    display: BytesDisplayVariant,
}

impl ProfileEntry {
    fn draw(&mut self, ui: &mut egui::Ui) {}
}

fn default_profiles() -> BTreeMap<String, ProfileEntry> {
    let mut profiles = BTreeMap::new();
    profiles.insert(
        DRIVE.to_owned(),
        ProfileEntry {
            key: ProfileEntryKey::Root,
            alias: "Root tree".to_owned(),
            sub_items: vec![
                ProfileEntry {
                    key: vec![64].into(),
                    alias: "Data contract documents".to_string(),
                    sub_items: vec![],
                    display: BytesDisplayVariant::U8,
                },
                ProfileEntry {
                    key: vec![32].into(),
                    alias: "Identities".to_string(),
                    sub_items: vec![ProfileEntry {
                        key: ProfileEntryKey::Capture,
                        alias: "Identity {}".to_owned(),
                        sub_items: vec![],
                        display: BytesDisplayVariant::Hex,
                    }],
                    display: BytesDisplayVariant::U8,
                },
                ProfileEntry {
                    key: vec![24].into(),
                    alias: "Unique public key hashes to identities".to_string(),
                    sub_items: vec![],
                    display: BytesDisplayVariant::U8,
                },
                ProfileEntry {
                    key: vec![8].into(),
                    alias: "Non-unique public key Key hashes to identities".to_string(),
                    sub_items: vec![],
                    display: BytesDisplayVariant::U8,
                },
                ProfileEntry {
                    key: vec![48].into(),
                    alias: "Pools".to_string(),
                    sub_items: vec![],
                    display: BytesDisplayVariant::U8,
                },
                ProfileEntry {
                    key: vec![40].into(),
                    alias: "Pre funded specialized balances".to_string(),
                    sub_items: vec![],
                    display: BytesDisplayVariant::U8,
                },
                ProfileEntry {
                    key: vec![72].into(),
                    alias: "Spent asset lock transactions".to_string(),
                    sub_items: vec![],
                    display: BytesDisplayVariant::U8,
                },
                ProfileEntry {
                    key: vec![104].into(),
                    alias: "Misc".to_string(),
                    sub_items: vec![],
                    display: BytesDisplayVariant::U8,
                },
                ProfileEntry {
                    key: vec![80].into(),
                    alias: "Withdrawal transactions".to_string(),
                    sub_items: vec![],
                    display: BytesDisplayVariant::U8,
                },
                ProfileEntry {
                    key: vec![96].into(),
                    alias: "Balances".to_string(),
                    sub_items: vec![],
                    display: BytesDisplayVariant::U8,
                },
                ProfileEntry {
                    key: vec![16].into(),
                    alias: "Token balances".to_string(),
                    sub_items: vec![],
                    display: BytesDisplayVariant::U8,
                },
                ProfileEntry {
                    key: vec![120].into(),
                    alias: "Versions".to_string(),
                    sub_items: vec![],
                    display: BytesDisplayVariant::U8,
                },
                ProfileEntry {
                    key: vec![112].into(),
                    alias: "Votes".to_string(),
                    sub_items: vec![],
                    display: BytesDisplayVariant::U8,
                },
            ],
            display: BytesDisplayVariant::U8,
        },
    );

    profiles
}

pub(crate) struct ProfilesView {
    profiles: BTreeMap<String, ProfileEntry>,
    selected: String,
}

impl ProfilesView {
    pub(crate) fn persist(&self, storage: &mut dyn Storage) {
        if let Some(profiles_b64) = bincode::encode_to_vec(&self.profiles, bincode::config::standard())
            .ok()
            .map(|bytes| BASE64_STANDARD.encode(bytes))
        {
            storage.set_string(PROFILES_KEY, profiles_b64);
        }
    }

    pub(crate) fn restore(storage: Option<&dyn Storage>) -> Self {
        let profiles = storage
            .and_then(|s| s.get_string(PROFILES_KEY))
            .and_then(|param| BASE64_STANDARD.decode(param).ok())
            .and_then(|encoded| {
                bincode::decode_from_slice(&encoded, bincode::config::standard())
                    .map(|(result, _)| result)
                    .ok()
            })
            .unwrap_or_else(|| default_profiles());
        ProfilesView {
            profiles,
            selected: DRIVE.to_string(),
        }
    }

    pub(crate) fn draw(&mut self, ui: &mut egui::Ui) {
        for (key, _) in self.profiles.iter() {
            if ui.radio(self.selected.as_str() == key.as_str(), key).clicked() {
                self.selected = key.to_owned();
            };
        }

        ui.separator();

        let mut profile = self.profiles.get_mut(&self.selected);
    }
}
