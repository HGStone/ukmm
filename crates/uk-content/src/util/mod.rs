mod collections;
pub mod converts;

use std::{collections::BTreeMap, str::FromStr};

pub use collections::*;
use roead::{aamp::*, byml::Byml, types::FixedSafeString};

pub fn diff_plist<P: ParameterListing + From<ParameterList>>(base: &P, other: &P) -> P {
    ParameterList {
        lists:   other
            .lists()
            .0
            .iter()
            .filter_map(|(k, v)| {
                if !base.lists().0.contains_key(k) {
                    Some((*k, v.clone()))
                } else if base.lists().0[k] != *v {
                    Some((*k, diff_plist(&base.lists().0[k], v)))
                } else {
                    None
                }
            })
            .collect(),
        objects: other
            .objects()
            .0
            .iter()
            .filter_map(|(k, v)| {
                if !base.objects().0.contains_key(k) {
                    Some((*k, v.clone()))
                } else if base.objects().0[k] != *v {
                    Some((*k, diff_pobj(&base.objects().0[k], v)))
                } else {
                    None
                }
            })
            .collect(),
    }
    .into()
}

pub fn diff_pobj(base: &ParameterObject, other: &ParameterObject) -> ParameterObject {
    other
        .0
        .iter()
        .filter_map(|(k, v)| {
            if !base.0.contains_key(k) || base.0[k] != *v {
                Some((*k, v.clone()))
            } else {
                None
            }
        })
        .collect()
}

pub fn merge_plist<P: ParameterListing + From<ParameterList>>(base: &P, diff: &P) -> P {
    ParameterList {
        objects: {
            let mut new = base.objects().clone();
            for (k, v) in &diff.objects().0 {
                if !new.0.contains_key(k) {
                    new.0.insert(*k, v.clone());
                } else {
                    new.0[k] = merge_pobj(&new.0[k], v);
                }
            }
            new
        },
        lists:   {
            let mut new = base.lists().clone();
            for (k, v) in &diff.lists().0 {
                if !new.0.contains_key(k) {
                    new.0.insert(*k, v.clone());
                } else {
                    new.0[k] = merge_plist(&new.0[k], v);
                }
            }
            new
        },
    }
    .into()
}

pub fn merge_pobj(base: &ParameterObject, diff: &ParameterObject) -> ParameterObject {
    base.0
        .iter()
        .chain(diff.0.iter())
        .map(|(k, v)| (*k, v.clone()))
        .collect()
}

pub fn diff_byml_shallow(base: &Byml, other: &Byml) -> Byml {
    if let Byml::Hash(base) = &base && let &Byml::Hash(other) = &other {
        Byml::Hash(other.iter().filter_map(|(key, value)| {
            if base.get(key) != Some(value) {
                Some((key.clone(), value.clone()))
            } else {
                None
            }
        }).chain(
            base.keys().filter_map(|key| (!other.contains_key(key)).then(|| (key.clone(), Byml::Null)))
        ).collect())
    } else {
        panic!("Can only shallow diff BYML hashes")
    }
}

pub fn merge_byml_shallow(base: &Byml, diff: &Byml) -> Byml {
    match (base, diff) {
        (Byml::Hash(base), Byml::Hash(diff)) => {
            Byml::Hash(
                base.iter()
                    .chain(diff.iter())
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            )
        }
        (Byml::Hash(base), Byml::Null) => Byml::Hash(base.clone()),
        _ => panic!("Can only shallow merge BYML hashes"),
    }
}

pub fn simple_index_diff<T: Clone + PartialEq>(
    base: &BTreeMap<usize, T>,
    other: &BTreeMap<usize, T>,
) -> BTreeMap<usize, T> {
    other
        .iter()
        .filter_map(|(i, other_item)| {
            (base.get(i) != Some(other_item)).then(|| (*i, other_item.clone()))
        })
        .collect()
}

pub fn simple_index_merge<T: Clone + PartialEq>(
    base: &BTreeMap<usize, T>,
    diff: &BTreeMap<usize, T>,
) -> BTreeMap<usize, T> {
    base.iter()
        .chain(diff.iter())
        .map(|(i, body)| (*i, body.clone()))
        .collect()
}

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Hash,
    Ord,
    PartialOrd,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct BymlHashValue(pub u32);

impl TryFrom<Byml> for BymlHashValue {
    type Error = crate::UKError;

    fn try_from(value: Byml) -> crate::Result<Self> {
        Ok(match value {
            Byml::U32(v) => Self(v),
            Byml::I32(v) => Self(u32::from_le_bytes(v.to_le_bytes())),
            _ => {
                return Err(crate::UKError::WrongBymlType(
                    "not an integer".into(),
                    "an integer",
                ));
            }
        })
    }
}

impl TryFrom<&Byml> for BymlHashValue {
    type Error = crate::UKError;

    fn try_from(value: &Byml) -> crate::Result<Self> {
        Ok(match value {
            Byml::U32(v) => Self(*v),
            Byml::I32(v) => Self(u32::from_le_bytes(v.to_le_bytes())),
            _ => {
                return Err(crate::UKError::WrongBymlType(
                    "not an integer".into(),
                    "an integer",
                ));
            }
        })
    }
}

impl FromStr for BymlHashValue {
    type Err = crate::UKError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u32>()
            .map_err(|_| crate::UKError::Other("Invalid BYML key"))
            .map(|h| h.into())
    }
}

impl From<u32> for BymlHashValue {
    fn from(val: u32) -> Self {
        Self(val)
    }
}

impl From<i32> for BymlHashValue {
    fn from(val: i32) -> Self {
        Self(u32::from_le_bytes(val.to_le_bytes()))
    }
}

impl From<usize> for BymlHashValue {
    fn from(val: usize) -> Self {
        Self(val as u32)
    }
}

impl From<BymlHashValue> for u32 {
    fn from(val: BymlHashValue) -> Self {
        val.0
    }
}

impl From<BymlHashValue> for i32 {
    fn from(val: BymlHashValue) -> Self {
        val.0 as i32
    }
}

impl From<BymlHashValue> for Byml {
    fn from(val: BymlHashValue) -> Self {
        if val.0 >= 0x80000000 {
            Byml::U32(val.0)
        } else {
            Byml::I32(i32::from_le_bytes(val.0.to_le_bytes()))
        }
    }
}

impl From<&BymlHashValue> for Byml {
    fn from(val: &BymlHashValue) -> Self {
        if val.0 >= 0x80000000 {
            Byml::U32(val.0)
        } else {
            Byml::I32(i32::from_le_bytes(val.0.to_le_bytes()))
        }
    }
}

/// Adapted from https://github.com/bluss/maplit/blob/master/src/lib.rs
#[macro_export]
macro_rules! bhash {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(bhash!(@single $rest)),*]));

    ($($key:expr => $value:expr,)+) => { bhash!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            let _cap = bhash!(@count $($key),*);
            let mut _map = ::roead::byml::Hash::default();
            _map.reserve(_cap);

            $(
                let _ = _map.insert(::smartstring::alias::String::from($key), $value);
            )*
            ::roead::byml::Byml::Hash(_map)
        }
    };
}
pub use bhash;

/// Adapted from https://github.com/bluss/maplit/blob/master/src/lib.rs
#[macro_export]
macro_rules! params {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(params!(@single $rest)),*]));

    ($($key:expr => $value:expr,)+) => { params!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            let _cap = params!(@count $($key),*);
            let mut _map = ::indexmap::IndexMap::<::roead::aamp::Name, ::roead::aamp::Parameter, ::std::hash::BuildHasherDefault<::rustc_hash::FxHasher>>::default();
            _map.reserve(_cap);

            $(
                let _ = _map.insert($key.into(), $value);
            )*
            ::roead::aamp::ParameterObject(_map)
        }
    };
}
pub use params;

/// Adapted from https://github.com/bluss/maplit/blob/master/src/lib.rs
#[macro_export]
macro_rules! pobjs {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(pobjs!(@single $rest)),*]));

    ($($key:expr => $value:expr,)+) => { pobjs!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            let _cap = pobjs!(@count $($key),*);
            let mut _map = ::indexmap::IndexMap::<::roead::aamp::Name, ::roead::aamp::ParameterObject, ::std::hash::BuildHasherDefault<::rustc_hash::FxHasher>>::default();
            _map.reserve(_cap);

            $(
                let _ = _map.insert($key.into(), $value);
            )*
            ::roead::aamp::ParameterObjectMap(_map)
        }
    };
}
pub use pobjs;

/// Adapted from https://github.com/bluss/maplit/blob/master/src/lib.rs
#[macro_export]
macro_rules! plists {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(plists!(@single $rest)),*]));

    ($($key:expr => $value:expr,)+) => { plists!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            let _cap = plists!(@count $($key),*);
            let mut _map = ::indexmap::IndexMap::<::roead::aamp::Name, ::roead::aamp::ParameterList, ::std::hash::BuildHasherDefault<::rustc_hash::FxHasher>>::default();
            _map.reserve(_cap);

            $(
                let _ = _map.insert($key.into(), $value);
            )*
            ::roead::aamp::ParameterListMap(_map)
        }
    };
}
pub use plists;

pub trait ParameterExt {
    fn as_safe_string<const N: usize>(&self) -> roead::Result<FixedSafeString<N>>;
}

impl ParameterExt for Parameter {
    fn as_safe_string<const N: usize>(&self) -> roead::Result<FixedSafeString<N>> {
        match self {
            Self::String32(s) => Ok(s.as_str().into()),
            Self::String64(s) => Ok(s.as_str().into()),
            Self::String256(s) => Ok(s.as_str().into()),
            Self::StringRef(s) => Ok(s.as_str().into()),
            _ => {
                Err(roead::Error::TypeError(
                    format!("{self:#?}").into(),
                    "a string",
                ))
            }
        }
    }
}

#[cfg(test)]
#[test]
fn test_any_safe_string() {
    let ss1 = Parameter::String32(
        "Bob and I go back. Way back. Back before ludicrous seat belt laws.".into(),
    );
    let ss2 = Parameter::String64(Box::new("jimmy".into()));
    let ss3 = Parameter::String256(Box::new("interesting indeed my friend".into()));
    let ss4 = Parameter::StringRef("A string".into());
    assert_eq!(
        ss1.as_safe_string::<64>().unwrap(),
        "Bob and I go back. Way back. Bac".into()
    );
    assert_eq!(ss2.as_safe_string::<64>().unwrap(), "jimmy".into());
    assert_eq!(
        ss3.as_safe_string::<32>().unwrap(),
        "interesting indeed my friend".into()
    );
    assert_eq!(ss4.as_safe_string::<32>().unwrap(), "A string".into());
}

pub trait IteratorExt
where
    Self: Sized,
{
    fn named_enumerate(self, name: &str) -> NamedEnumerate<'_, Self> {
        NamedEnumerate::new(self, name)
    }
}

impl<T> IteratorExt for T where T: Iterator {}

pub struct NamedEnumerate<'a, I> {
    iter:   I,
    count:  usize,
    name:   &'a str,
    buffer: Vec<u8>,
}

impl<'a, I> NamedEnumerate<'a, I> {
    pub(crate) fn new(iter: I, name: &'a str) -> NamedEnumerate<'a, I> {
        NamedEnumerate {
            iter,
            count: 0,
            name,
            buffer: {
                let mut vec = Vec::with_capacity(u16::MAX as usize);
                vec.extend(name.as_bytes());
                vec
            },
        }
    }
}

impl<'a, I> Iterator for NamedEnumerate<'a, I>
where
    I: Iterator,
{
    type Item = (String, <I as Iterator>::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next()?;
        let i = self.count;
        self.count += 1;
        let name_len = self.name.len();
        let name = unsafe {
            self.buffer.set_len(u16::MAX as usize);
            let len = lexical_core::write_unchecked(i as u16, &mut self.buffer[name_len..]).len();
            self.buffer.set_len(len + name_len);
            String::from_utf8_unchecked(self.buffer.clone())
        };
        Some((name, item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}