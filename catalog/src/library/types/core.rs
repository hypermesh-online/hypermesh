// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Core library types - package structs, serde helpers, and basic type definitions

use serde::Serializer;
use std::collections::HashMap;
use std::sync::Arc;

// Serde support for Arc<str>
pub(crate) mod arc_str_serde {
    use super::*;
    use serde::de::{Deserializer, Visitor};
    use std::fmt;

    pub fn serialize<S>(value: &Arc<str>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(value)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<str>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArcStrVisitor;
        impl<'de> Visitor<'de> for ArcStrVisitor {
            type Value = Arc<str>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Arc::from(v))
            }
            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Arc::from(v))
            }
        }
        deserializer.deserialize_string(ArcStrVisitor)
    }
}

// Serde support for Option<Arc<str>>
pub(crate) mod option_arc_str_serde {
    use super::*;
    use serde::de::{Deserializer, Visitor};
    use std::fmt;

    pub fn serialize<S>(value: &Option<Arc<str>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(s) => serializer.serialize_some(s.as_ref()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Arc<str>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OptArcStrVisitor;
        impl<'de> Visitor<'de> for OptArcStrVisitor {
            type Value = Option<Arc<str>>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an optional string")
            }
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(None)
            }
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                super::arc_str_serde::deserialize(deserializer).map(Some)
            }
        }
        deserializer.deserialize_option(OptArcStrVisitor)
    }
}

// Serde support for Arc<[Arc<str>]>
pub(crate) mod arc_slice_arc_str_serde {
    use super::*;
    use serde::de::{Deserializer, SeqAccess, Visitor};
    use std::fmt;

    pub fn serialize<S>(value: &Arc<[Arc<str>]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(value.len()))?;
        for item in value.iter() {
            seq.serialize_element(item.as_ref())?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<[Arc<str>]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArcSliceVisitor;
        impl<'de> Visitor<'de> for ArcSliceVisitor {
            type Value = Arc<[Arc<str>]>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence of strings")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = seq.next_element::<String>()? {
                    vec.push(Arc::from(elem.as_str()));
                }
                Ok(Arc::from(vec.into_boxed_slice()))
            }
        }
        deserializer.deserialize_seq(ArcSliceVisitor)
    }
}

// Generic serde for Arc<[T]> where T: Serialize + Deserialize
macro_rules! arc_slice_serde {
    ($name:ident, $ty:ty) => {
        pub(crate) mod $name {
            use super::*;
            use serde::de::{Deserializer, SeqAccess, Visitor};
            use std::fmt;

            pub fn serialize<S>(value: &Arc<[$ty]>, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                use serde::ser::SerializeSeq;
                let mut seq = serializer.serialize_seq(Some(value.len()))?;
                for item in value.iter() {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }

            pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<[$ty]>, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct ArcSliceVisitor;
                impl<'de> Visitor<'de> for ArcSliceVisitor {
                    type Value = Arc<[$ty]>;
                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a sequence")
                    }
                    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: SeqAccess<'de>,
                    {
                        let mut vec = Vec::new();
                        while let Some(elem) = seq.next_element::<$ty>()? {
                            vec.push(elem);
                        }
                        Ok(Arc::from(vec.into_boxed_slice()))
                    }
                }
                deserializer.deserialize_seq(ArcSliceVisitor)
            }
        }
    };
}

arc_slice_serde!(arc_slice_dependency_serde, PackageDependency);
arc_slice_serde!(arc_slice_contentref_serde, ContentRef);
arc_slice_serde!(arc_slice_binaryref_serde, BinaryRef);
arc_slice_serde!(arc_slice_templateparam_serde, TemplateParameter);

// Serde support for Arc<HashMap<Arc<str>, Arc<str>>>
pub(crate) mod arc_hashmap_arc_str_serde {
    use super::*;
    use serde::de::{Deserializer, MapAccess, Visitor};
    use std::fmt;

    pub fn serialize<S>(value: &Arc<HashMap<Arc<str>, Arc<str>>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (k, v) in value.iter() {
            map.serialize_entry(k.as_ref(), v.as_ref())?;
        }
        map.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<HashMap<Arc<str>, Arc<str>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArcHashMapVisitor;
        impl<'de> Visitor<'de> for ArcHashMapVisitor {
            type Value = Arc<HashMap<Arc<str>, Arc<str>>>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map of strings to strings")
            }
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut hashmap = HashMap::new();
                while let Some((key, value)) = map.next_entry::<String, String>()? {
                    hashmap.insert(Arc::from(key.as_str()), Arc::from(value.as_str()));
                }
                Ok(Arc::new(hashmap))
            }
        }
        deserializer.deserialize_map(ArcHashMapVisitor)
    }
}

// Performance optimization: Pre-allocated common strings
lazy_static::lazy_static! {
    pub static ref EMPTY_STR: Arc<str> = Arc::from("");
    pub static ref DEFAULT_VERSION: Arc<str> = Arc::from("1.0.0");
    pub static ref DEFAULT_LICENSE: Arc<str> = Arc::from("MIT");
}

// Import types that serde macros reference (used in macro expansion)
use super::metadata::{PackageDependency, ContentRef, BinaryRef, TemplateParameter};
