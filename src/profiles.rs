use std::collections::BTreeMap;

use base64::prelude::*;
use bincode::{Decode, Encode};
use eframe::{egui, Storage};

use crate::PROFILES_KEY;

/// I drive
const DRIVE: &'static str = "drive";

#[derive(Encode, Decode)]
struct Profile {
    entries: Vec<ProfileEntry>,
}

#[derive(Encode, Decode)]
struct ProfileEntry {}

fn default_profiles() -> BTreeMap<String, Profile> {
    let mut profiles = BTreeMap::new();
    profiles.insert(DRIVE.to_owned(), Profile { entries: vec![] });

    profiles
}

pub(crate) struct ProfilesView {
    profiles: BTreeMap<String, Profile>,
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
