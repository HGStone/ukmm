use crate::{prelude::*, Result, UKError};
use join_str::jstr;
use msyt::{
    model::{Entry, MsbtInfo},
    Endianness, Msyt,
};
use roead::sarc::{Sarc, SarcWriter};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

impl Mergeable for Msyt {
    fn diff(&self, other: &Self) -> Self {
        Self {
            msbt: self.msbt.clone(),
            entries: other
                .entries
                .iter()
                .filter_map(|(k, v)| {
                    (self.entries.get(k) != Some(v)).then(|| (k.clone(), v.clone()))
                })
                .collect(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        let entries: indexmap::IndexMap<String, Entry> = self
            .entries
            .iter()
            .chain(diff.entries.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Self {
            msbt: MsbtInfo {
                group_count: entries.len() as u32,
                atr1_unknown: self.msbt.atr1_unknown,
                ato1: self.msbt.ato1.clone(),
                nli1: self.msbt.nli1.clone(),
                tsy1: self.msbt.tsy1.clone(),
            },
            entries,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MessagePack(pub BTreeMap<String, Msyt>);

impl Mergeable for MessagePack {
    fn diff(&self, other: &Self) -> Self {
        Self(
            other
                .0
                .iter()
                .filter_map(|(file, text)| {
                    if let Some(self_text) = self.0.get(file) {
                        if self_text != text {
                            Some((file.clone(), self_text.diff(text)))
                        } else {
                            None
                        }
                    } else {
                        Some((file.clone(), text.clone()))
                    }
                })
                .collect(),
        )
    }

    fn merge(&self, diff: &Self) -> Self {
        let files = self
            .0
            .keys()
            .chain(diff.0.keys())
            .cloned()
            .collect::<BTreeSet<String>>();
        Self(
            files
                .into_iter()
                .map(|file| match (self.0.get(&file), diff.0.get(&file)) {
                    (Some(self_text), Some(diff_text)) => {
                        (file.clone(), self_text.merge(diff_text))
                    }
                    (v1, v2) => (file.clone(), unsafe {
                        // We know this is sound because the key came from an entry
                        // in one of these two maps.
                        v2.or(v1).cloned().unwrap_unchecked()
                    }),
                })
                .collect(),
        )
    }
}

impl TryFrom<&'_ Sarc<'_>> for MessagePack {
    type Error = UKError;

    fn try_from(sarc: &Sarc<'_>) -> Result<Self> {
        Ok(Self(
            sarc.files()
                .map(|file| -> Result<(String, Msyt)> {
                    let name = file.name_unchecked().trim_end_matches(".msbt").to_owned();
                    Ok((name, Msyt::from_msbt_bytes(file.data())?))
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl MessagePack {
    pub fn into_sarc_writer(self, endian: Endian) -> SarcWriter {
        SarcWriter::new(endian.into()).with_files(self.0.into_iter().map(|(name, text)| {
            (
                jstr!("{&name}.msbt"),
                text.into_msbt_bytes(match endian {
                    Endian::Little => Endianness::Little,
                    Endian::Big => Endianness::Big,
                })
                .unwrap(),
            )
        }))
    }
}

impl Resource for MessagePack {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&Sarc::read(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: Endian) -> roead::Bytes {
        self.into_sarc_writer(endian).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref()
            .file_stem()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("Msg_"))
            .unwrap_or(false)
    }
}