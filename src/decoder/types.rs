use alloc::{borrow::ToOwned, collections::BTreeMap, string::String, vec::Vec};
use serde_json::Value;

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

#[cfg(test)]
impl TraitSchema {
    #[allow(dead_code)]
    pub fn new(
        name: &str,
        type_: ImageType,
        dob0_trait: &str,
        pattern: Pattern,
        args: Option<BTreeMap<DOB0TraitValue, String>>,
    ) -> Self {
        Self {
            name: name.to_owned(),
            type_,
            dob0_trait: dob0_trait.to_owned(),
            pattern,
            args,
        }
    }

    #[allow(dead_code)]
    pub fn encode(&self) -> Vec<Value> {
        let mut values = vec![
            Value::String(self.name.clone()),
            Value::String(match self.type_ {
                ImageType::ColorCode => "color".to_owned(),
                ImageType::URI => "uri".to_owned(),
                ImageType::RawImage => "raw".to_owned(),
            }),
            Value::String(self.dob0_trait.clone()),
            Value::String(match self.pattern {
                Pattern::Options => "options".to_owned(),
                Pattern::Range => "range".to_owned(),
                Pattern::Raw => "raw".to_owned(),
            }),
        ];
        if let Some(args) = &self.args {
            let item = Value::Array(
                args.iter()
                    .map(|(key, value)| {
                        let mut item = Vec::new();
                        match key {
                            DOB0TraitValue::String(name) => {
                                item.push(Value::String(name.clone()));
                            }
                            DOB0TraitValue::Number(number) => {
                                item.push(Value::Number((*number).into()));
                            }
                            DOB0TraitValue::Range(start, end) => {
                                item.push(Value::Array(vec![(*start).into(), (*end).into()]));
                            }
                            DOB0TraitValue::Any => {
                                item.push(Value::Array(vec!["*".into()]));
                            }
                        }
                        item.push(Value::String(value.clone()));
                        Value::Array(item)
                    })
                    .collect(),
            );
            values.push(item);
        }
        values
    }
}

pub fn decode_trait_schema(traits_pool: Vec<Vec<Value>>) -> Result<Vec<TraitSchema>, Error> {
    let traits_base = traits_pool
        .into_iter()
        .map(|schema| {
            if schema.len() < 4 {
                return Err(Error::SchemaInsufficientElements);
            }
            let name = schema[0].as_str().ok_or(Error::SchemaInvalidName)?;
            let type_ = match schema[1].as_str().ok_or(Error::SchemaInvalidType)? {
                "color" => ImageType::ColorCode,
                "uri" => ImageType::URI,
                "image" => ImageType::RawImage,
                _ => return Err(Error::SchemaTypeMismatch),
            };
            let dob0_trait = schema[2].as_str().ok_or(Error::SchemaInvalidTraitName)?;
            let pattern_str = schema[3].as_str().ok_or(Error::SchemaInvalidPattern)?;
            let pattern = match (pattern_str, &type_) {
                ("options", ImageType::ColorCode | ImageType::URI) => Pattern::Options,
                ("range", ImageType::ColorCode | ImageType::URI) => Pattern::Range,
                ("raw", ImageType::RawImage | ImageType::URI) => Pattern::Raw,
                _ => return Err(Error::SchemaPatternMismatch),
            };
            let args = if let Some(args) = schema.get(4) {
                let args = args
                    .as_array()
                    .ok_or(Error::SchemaInvalidArgs)?
                    .iter()
                    .map(|value| {
                        let item = value.as_array().ok_or(Error::SchemaInvalidArgsElement)?;
                        let (Some(trait_pattern), Some(dob1_value)) = (item.first(), item.get(1))
                        else {
                            return Err(Error::SchemaInvalidArgsElement);
                        };
                        let key = if trait_pattern.is_number() {
                            DOB0TraitValue::Number(trait_pattern.as_u64().unwrap())
                        } else if trait_pattern.is_string() {
                            DOB0TraitValue::String(trait_pattern.as_str().unwrap().to_owned())
                        } else if trait_pattern.is_array() {
                            let range = trait_pattern.as_array().unwrap();
                            if Some(Some("*")) == range.first().map(|v| v.as_str()) {
                                DOB0TraitValue::Any
                            } else {
                                if range.len() != 2 {
                                    return Err(Error::SchemaInvalidArgsElement);
                                }
                                DOB0TraitValue::Range(
                                    range[0].as_u64().ok_or(Error::SchemaInvalidArgsElement)?,
                                    range[1].as_u64().ok_or(Error::SchemaInvalidArgsElement)?,
                                )
                            }
                        } else {
                            return Err(Error::SchemaInvalidArgsElement);
                        };
                        let value = dob1_value
                            .as_str()
                            .ok_or(Error::SchemaInvalidArgsElement)?
                            .to_owned();
                        Ok((key, value))
                    })
                    .collect::<Result<BTreeMap<_, _>, _>>()?;
                Some(args)
            } else {
                None
            };
            Ok(TraitSchema {
                name: name.to_owned(),
                type_,
                dob0_trait: dob0_trait.to_owned(),
                pattern,
                args,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(traits_base)
}

// use `test_generate_basic_example` test case in spore-dob-0 repo to generate the following test
#[test]
fn test_basic_trait_schema_encode_decode() {
    let traits = vec![
        TraitSchema::new(
            "0",
            ImageType::ColorCode,
            "Name",
            Pattern::Options,
            Some(
                vec![
                    (
                        DOB0TraitValue::String("Ethan".to_owned()),
                        "#FF0000".to_owned(),
                    ),
                    (
                        DOB0TraitValue::String("Alice".to_owned()),
                        "#0000FF".to_owned(),
                    ),
                    (
                        DOB0TraitValue::String("Bob".to_owned()),
                        "#00FF00".to_owned(),
                    ),
                    (
                        DOB0TraitValue::Any,
                        "#FFFFFF".to_owned(),
                    )
                ]
                .into_iter()
                .collect(),
            ),
        ),
        TraitSchema::new(
            "0",
            ImageType::URI,
            "Age",
            Pattern::Range,
            Some(
                vec![
                    (
                        DOB0TraitValue::Range(0, 50),
                        "btcfs://b2f4560f17679d3e3fca66209ac425c660d28a252ef72444c3325c6eb0364393i0".to_owned()
                    ),
                    (
                        DOB0TraitValue::Range(51, 100),
                        "btcfs://eb3910b3e32a5ed9460bd0d75168c01ba1b8f00cc0faf83e4d8b67b48ea79676i0".to_owned(),
                    ),
                    (
                        DOB0TraitValue::Any,
                        "btcfs://11b6303eb7d887d7ade459ac27959754cd55f9f9e50345ced8e1e8f47f4581fai0".to_owned(),
                    )
                ]
                .into_iter()
                .collect(),
            ),
        ),
        TraitSchema::new(
            "0",
            ImageType::URI,
            "Score",
            Pattern::Range,
            Some(
                vec![
                    (
                        DOB0TraitValue::Range(0, 1000),
                        "btcfs://11d6cc654f4c0759bfee520966937a4304db2b33880c88c2a6c649e30c7b9aaei0".to_owned()
                    ),
                    (
                        DOB0TraitValue::Any,
                        "btcfs://e1484915b27e45b120239080fe5032580550ff9ff759eb26ee86bf8aaf90068bi0".to_owned(),
                    )
                ]
                .into_iter()
                .collect(),
            ),
        ),
        TraitSchema::new(
            "1",
            ImageType::URI,
            "Value",
            Pattern::Range,
            Some(
                vec![
                    (
                        DOB0TraitValue::Range(0, 100000),
                        "btcfs://11d6cc654f4c0759bfee520966937a4304db2b33880c88c2a6c649e30c7b9aaei0".to_owned()
                    ),
                    (
                        DOB0TraitValue::Any,
                        "btcfs://e1484915b27e45b120239080fe5032580550ff9ff759eb26ee86bf8aaf90068bi0".to_owned(),
                    )
                ]
                .into_iter()
                .collect(),
            ),
        ),
    ];
    let encoded = traits.iter().map(TraitSchema::encode).collect::<Vec<_>>();
    println!("{}\n", serde_json::to_string_pretty(&encoded).unwrap());
    println!("pattern = {}", serde_json::to_string(&encoded).unwrap());
    let decoded = decode_trait_schema(encoded).expect("decode");
    assert_eq!(traits, decoded);
}
