use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{DocMeta, Repo};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MainMetadata {
    pub items: HashMap<i64, MetaItem>,
    pub books: HashMap<i64, Repo>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct BackupTime(#[serde(with = "time::serde::iso8601")] OffsetDateTime);

#[derive(Debug, Serialize, Deserialize)]
pub struct MetaItem {
    pub last_updated: BackupTime,
    pub backups: Vec<BackupTime>,
}

impl MainMetadata {
    /// Whether document with the given metadata needs a new backup.
    pub fn needs_backup(&self, meta: &DocMeta<'_>) -> bool {
        !self
            .items
            .get(&meta.raw.id)
            .is_some_and(|m| m.last_updated.0 >= meta.raw.updated_at)
    }

    /// Tracks the backed-up metadata.
    pub fn track_backup(&mut self, meta: &DocMeta<'_>) {
        let time = BackupTime(meta.raw.updated_at);
        if let Some(m) = self.items.get_mut(&meta.raw.id) {
            m.last_updated = time;
            m.backups.push(time);
        } else {
            self.items.insert(
                meta.raw.id,
                MetaItem {
                    last_updated: time,
                    backups: vec![time],
                },
            );
        }
    }
}
