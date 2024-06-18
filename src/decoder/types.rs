use alloc::{collections::BTreeMap, string::String, vec::Vec};

#[repr(u64)]
#[cfg_attr(test, derive(Debug))]
pub enum Error {
    ParseInvalidArgCount = 1,
    ParseInvalidDOB0Output,
    ParseInvalidTraitsBase,

    SchemaInsufficientElements,
    SchemaInvalidName,
    SchemaInvalidTraitName,
    SchemaInvalidType,
    SchemaTypeMismatch,
    SchemaInvalidPattern,
    SchemaPatternMismatch,
    SchemaInvalidArgs,
    SchemaInvalidArgsElement,
    SchemaInvalidParsedTraitType,

    DecodeInvalidOptionArgs,
    DecodeInvalidRawValue,
    DecodeBadUTF8Format,
    DecodeBadColorCodeFormat,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub enum ParsedTrait {
    String(String),
    Number(u64),
}

impl ParsedTrait {
    pub fn get_string(&self) -> Result<&String, Error> {
        if let ParsedTrait::String(value) = self {
            Ok(value)
        } else {
            Err(Error::SchemaInvalidParsedTraitType)
        }
    }

    pub fn get_number(&self) -> Result<u64, Error> {
        if let ParsedTrait::Number(value) = self {
            Ok(*value)
        } else {
            Err(Error::SchemaInvalidParsedTraitType)
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct DOB0Output {
    pub name: String,
    pub traits: Vec<ParsedTrait>,
}

#[cfg_attr(test, derive(serde::Deserialize))]
pub struct Parameters {
    pub dob0_output: Vec<DOB0Output>,
    pub images_base: Vec<TraitSchema>,
}

#[derive(serde::Serialize)]
pub struct Image {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub content: String,
}

#[derive(serde::Serialize)]
pub struct DOB1Output {
    pub traits: Vec<DOB0Output>,
    pub images: Vec<Image>,
}

#[cfg_attr(test, derive(serde::Serialize, Clone, Debug))]
#[derive(serde::Deserialize, PartialEq, Eq)]
pub enum ImageType {
    ColorCode,
    URI,
    RawImage,
}

#[cfg_attr(test, derive(serde::Serialize, Clone, PartialEq, Debug))]
#[derive(serde::Deserialize)]
pub enum Pattern {
    Options,
    Range,
    Raw,
}

#[cfg_attr(test, derive(serde::Serialize, Clone, Debug))]
#[derive(serde::Deserialize, PartialOrd, PartialEq, Eq, Ord)]
pub enum DOB0TraitValue {
    String(String),
    Number(u64),
    Range(u64, u64),
    Any,
}

#[cfg_attr(test, derive(serde::Serialize, Clone, PartialEq, Debug))]
#[derive(serde::Deserialize)]
pub struct TraitSchema {
    pub name: String,
    pub type_: ImageType,
    pub dob0_trait: String,
    pub pattern: Pattern,
    pub args: Option<BTreeMap<DOB0TraitValue, String>>,
}
