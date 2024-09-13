use std::borrow::Borrow;

use eframe::{
    egui::{self, Frame, Label, Margin, TextEdit},
    Storage,
};
use serde::{Deserialize, Serialize};

use crate::{
    bus::{CommandBus, UserAction},
    bytes_utils::{bytes_by_display_variant, BytesDisplayVariant, BytesInput},
    path_ctx::{Path, PathCtx},
    PROFILES_KEY,
};

/// I drive
const DRIVE: &'static str = "drive";

#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Default)]
enum ProfileEntryKey {
    Key(BytesInput),
    #[default]
    Capture,
}

impl From<Vec<u8>> for ProfileEntryKey {
    fn from(value: Vec<u8>) -> Self {
        ProfileEntryKey::Key(BytesInput::new_from_bytes(value))
    }
}

impl ProfileEntryKey {
    fn draw(&mut self, ui: &mut egui::Ui, read_only: bool) {
        if read_only {
            match self {
                ProfileEntryKey::Key(bytes) => ui.label(format!("Key: {}", bytes.current_input())),
                ProfileEntryKey::Capture => ui.label("Capture"),
            };
        } else {
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
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct ProfileEntry {
    key: ProfileEntryKey,
    alias: String,
    sub_items: Vec<ProfileEntry>,
    display: BytesDisplayVariant,
    collapsed: bool,
}

type ToDelete = bool;

impl ProfileEntry {
    fn draw<'pa>(
        &mut self,
        ui: &mut egui::Ui,
        bus: &CommandBus<'pa>,
        read_only: bool,
        parent_path: Option<Path<'pa>>,
    ) -> ToDelete {
        let mut to_delete = false;
        let self_path = parent_path.and_then(|p| key_as_alias(&self.key).map(|k| p.child(k)));

        if self.collapsed {
            ui.horizontal(|line| {
                let icon = if read_only {
                    egui_phosphor::variants::regular::ARROW_FAT_LINES_DOWN
                } else {
                    egui_phosphor::variants::regular::PENCIL
                };
                if line.button(icon).on_hover_text("Expand profile entry").clicked() {
                    self.collapsed = false;
                }

                if let Some(path) = self_path {
                    if line
                        .button(egui_phosphor::regular::MAGNIFYING_GLASS)
                        .on_hover_text("Jump to subtree")
                        .clicked()
                    {
                        bus.user_action(UserAction::FocusSubtree(path));
                    }
                }

                line.label(&self.alias);
            });
        } else {
            let expanded_entry_indent = ui
                .horizontal(|line| {
                    let first_button_response =
                        line.button(egui_phosphor::variants::regular::ARROW_FAT_LINES_LEFT);
                    let first_button_right_border = first_button_response.rect.right();

                    if first_button_response
                        .on_hover_text("Collapse profile entry")
                        .clicked()
                    {
                        self.collapsed = true;
                    }

                    if let Some(path) = self_path {
                        if line
                            .button(egui_phosphor::regular::MAGNIFYING_GLASS)
                            .on_hover_text("Jump to subtree")
                            .clicked()
                        {
                            bus.user_action(UserAction::FocusSubtree(path));
                        }
                    }

                    if line
                        .button(egui_phosphor::regular::TRASH_SIMPLE)
                        .on_hover_text("Delete profile entry")
                        .clicked()
                    {
                        to_delete = true;
                    }

                    line.label("Alias:");

                    line.add_enabled(!read_only, TextEdit::singleline(&mut self.alias));

                    if !read_only {
                        if line
                            .button(egui_phosphor::variants::regular::PLUS_SQUARE)
                            .on_hover_text("Add sub item")
                            .clicked()
                        {
                            self.sub_items.push(ProfileEntry::default());
                        }
                    }

                    first_button_right_border - line.max_rect().left()
                })
                .inner;

            Frame::none()
                .outer_margin(Margin {
                    left: expanded_entry_indent,
                    ..Default::default()
                })
                .show(ui, |frame| {
                    self.key.draw(frame, read_only);

                    if matches!(self.key, ProfileEntryKey::Capture) {
                        if read_only {
                            frame.add_enabled(
                                false,
                                Label::new(format!("Show as: {}", self.display.as_ref())),
                            );
                        } else {
                            frame.collapsing("Captured value display", |collapsing| {
                                self.display.draw(collapsing);
                            });
                        }
                    }

                    draw_entries(frame, bus, &mut self.sub_items, read_only, self_path);
                });
        }

        to_delete
    }
}

fn key_as_alias(key: &ProfileEntryKey) -> Option<Vec<u8>> {
    match key {
        ProfileEntryKey::Key(k) => Some(k.get_bytes()),
        ProfileEntryKey::Capture => None,
    }
}

fn default_profiles() -> Vec<Profile> {
    let mut profiles = Vec::new();
    profiles.push(Profile {
        name: DRIVE.to_owned(),
        entries: vec![
            ProfileEntry {
                key: vec![64].into(),
                collapsed: true,
                alias: "Data contract documents".to_string(),
                sub_items: Vec::default(),
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                key: vec![32].into(),
                collapsed: true,
                alias: "Identities".to_string(),
                sub_items: vec![ProfileEntry {
                    key: ProfileEntryKey::Capture,
                    collapsed: true,
                    alias: "ID {}".to_owned(),
                    sub_items: Vec::default(),
                    display: BytesDisplayVariant::Hex,
                }],
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                key: vec![24].into(),
                collapsed: true,
                alias: "Unique public key hashes to identities".to_string(),
                sub_items: Vec::default(),
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                key: vec![8].into(),
                collapsed: true,
                alias: "Non-unique public key Key hashes to identities".to_string(),
                sub_items: Vec::default(),
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                key: vec![48].into(),
                collapsed: true,
                alias: "Pools".to_string(),
                sub_items: Vec::default(),
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                key: vec![40].into(),
                collapsed: true,
                alias: "Pre funded specialized balances".to_string(),
                sub_items: Vec::default(),
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                key: vec![72].into(),
                collapsed: true,
                alias: "Spent asset lock transactions".to_string(),
                sub_items: Vec::default(),
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                key: vec![104].into(),
                collapsed: true,
                alias: "Misc".to_string(),
                sub_items: Vec::default(),
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                key: vec![80].into(),
                collapsed: true,
                alias: "Withdrawal transactions".to_string(),
                sub_items: Vec::default(),
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                key: vec![96].into(),
                collapsed: true,
                alias: "Balances".to_string(),
                sub_items: Vec::default(),
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                key: vec![16].into(),
                collapsed: true,
                alias: "Token balances".to_string(),
                sub_items: Vec::default(),
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                key: vec![120].into(),
                collapsed: true,
                alias: "Versions".to_string(),
                sub_items: Vec::default(),
                display: BytesDisplayVariant::U8,
            },
            ProfileEntry {
                key: vec![112].into(),
                collapsed: true,
                alias: "Votes".to_string(),
                sub_items: Vec::default(),
                display: BytesDisplayVariant::U8,
            },
        ],
        read_only: true,
    });

    profiles
}

fn draw_entries<'pa>(
    ui: &mut egui::Ui,
    bus: &CommandBus<'pa>,
    entries: &mut Vec<ProfileEntry>,
    read_only: bool,
    parent_path: Option<Path<'pa>>,
) {
    let mut delete_idxs = Vec::new();

    for (idx, entry) in entries.iter_mut().enumerate() {
        let to_delete = entry.draw(ui, bus, read_only, parent_path);
        if to_delete {
            delete_idxs.push(idx);
        }
    }

    delete_idxs.reverse();
    for i in delete_idxs {
        entries.remove(i);
    }
}

#[derive(Serialize, Deserialize)]
struct Profile {
    name: String,
    entries: Vec<ProfileEntry>,
    read_only: bool,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ProfilesView {
    profiles: Vec<Profile>,
    selected: usize,
}

impl ProfilesView {
    pub(crate) fn persist(&self, storage: &mut dyn Storage) {
        if let Ok(s) = serde_json::to_string(self) {
            storage.set_string(PROFILES_KEY, s);
        }
    }

    pub(crate) fn restore(storage: Option<&dyn Storage>) -> Self {
        storage
            .and_then(|s| s.get_string(PROFILES_KEY))
            .and_then(|param| {
                serde_json::from_str(&param)
                    .inspect_err(|_| {
                        log::error!("Unable to restore profile settings, falling back to default")
                    })
                    .ok()
            })
            .unwrap_or_else(|| ProfilesView {
                profiles: default_profiles(),
                selected: 0,
            })
    }

    pub(crate) fn draw<'pa>(&mut self, ui: &mut egui::Ui, bus: &CommandBus<'pa>, path_ctx: &'pa PathCtx) {
        let mut selected_profile = None;
        let mut copied_profiles = Vec::new();
        let mut deleted_profiles = Vec::new();

        for (idx, profile) in self.profiles.iter_mut().enumerate() {
            let selected = self.selected == idx;

            ui.horizontal(|line| {
                if line.radio(selected, "").clicked() {
                    self.selected = idx;
                };

                line.text_edit_singleline(&mut profile.name);

                if line
                    .button(egui_phosphor::regular::COPY)
                    .on_hover_text("Make a profile copy")
                    .clicked()
                {
                    copied_profiles.push(Profile {
                        read_only: false,
                        name: format!("{} copy", profile.name),
                        entries: profile.entries.clone(),
                    });
                }

                if !profile.read_only
                    && line
                        .button(egui_phosphor::regular::TRASH_SIMPLE)
                        .on_hover_text("Delete profile")
                        .clicked()
                {
                    deleted_profiles.push(idx);
                }
            });
            if selected {
                selected_profile = Some(profile);
            }
        }

        ui.separator();

        if let Some(profile) = selected_profile {
            draw_entries(
                ui,
                bus,
                &mut profile.entries,
                profile.read_only,
                Some(path_ctx.get_root()),
            );

            if !profile.read_only && ui.button(egui_phosphor::regular::PLUS_SQUARE).clicked() {
                profile.entries.push(Default::default());
            }
        }

        self.profiles.append(&mut copied_profiles);

        for to_remove in deleted_profiles.iter() {
            self.profiles.remove(*to_remove);
            self.selected = self.selected.saturating_sub(deleted_profiles.len());
        }
    }

    pub(crate) fn active_profile_root_ctx(&self) -> RootActiveProfileContext {
        let profile = self.profiles.get(self.selected);
        RootActiveProfileContext::new(profile)
    }
}

pub(crate) struct RootActiveProfileContext<'pf>(ActiveProfileSubtreeContext<'pf>);

impl<'pf> Borrow<ActiveProfileSubtreeContext<'pf>> for RootActiveProfileContext<'pf> {
    fn borrow(&self) -> &ActiveProfileSubtreeContext<'pf> {
        &self.0
    }
}

impl<'pf> RootActiveProfileContext<'pf> {
    pub(crate) fn into_inner(self) -> ActiveProfileSubtreeContext<'pf> {
        self.0
    }

    pub(crate) fn fast_forward(self, path: Path) -> ActiveProfileSubtreeContext<'pf> {
        path.for_segments(|segments_iter| {
            let mut ctx = self.0;
            for s in segments_iter {
                ctx = ctx.child(s.bytes().to_vec());
            }
            ctx
        })
    }

    fn new(profile: Option<&'pf Profile>) -> Self {
        RootActiveProfileContext(ActiveProfileSubtreeContext {
            profile,
            entries: profile.map(|p| &p.entries),
            path_segments: Vec::new(),
        })
    }
}

pub(crate) struct ActiveProfileSubtreeContext<'pf> {
    profile: Option<&'pf Profile>,
    entries: Option<&'pf Vec<ProfileEntry>>,
    path_segments: Vec<Option<String>>,
}

impl<'pf> ActiveProfileSubtreeContext<'pf> {
    pub(crate) fn child(&self, key: Vec<u8>) -> Self {
        let mut path_segments = self.path_segments.clone();
        let mut idx = None;

        for (i, entry) in self.entries.into_iter().flatten().enumerate() {
            match &entry.key {
                ProfileEntryKey::Key(bytes) if bytes.get_bytes() == key => {
                    path_segments.push(Some(entry.alias.clone()));
                    idx = Some(i);
                    break;
                }
                ProfileEntryKey::Capture => {
                    path_segments.push(Some(
                        entry
                            .alias
                            .replace("{}", &bytes_by_display_variant(&key, &entry.display)),
                    ));
                    idx = Some(i);
                    break;
                }
                _ => {}
            }
        }

        if self.path_segments.len() == path_segments.len() {
            path_segments.push(None);
        }

        ActiveProfileSubtreeContext {
            profile: self.profile,
            entries: self
                .entries
                .and_then(|e| idx.and_then(|i| e.get(i)))
                .map(|e| &e.sub_items),
            path_segments,
        }
    }

    pub(crate) fn key_view(&self, key: &[u8]) -> Option<String> {
        self.entries
            .into_iter()
            .flatten()
            .find(|x| match &x.key {
                ProfileEntryKey::Key(bytes) => bytes.get_bytes() == key,
                ProfileEntryKey::Capture => true,
            })
            .map(|e| match e.key {
                ProfileEntryKey::Key(_) => e.alias.clone(),
                ProfileEntryKey::Capture => e.alias.replace("{}", &bytes_by_display_variant(key, &e.display)),
            })
    }

    pub(crate) fn path_segments_aliases(&self) -> &[Option<String>] {
        &self.path_segments
    }

    pub(crate) fn root_context(&self) -> RootActiveProfileContext {
        RootActiveProfileContext::new(self.profile)
    }
}
