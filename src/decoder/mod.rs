use alloc::{borrow::ToOwned, collections::BTreeMap, string::String, vec::Vec};

pub mod types;
use crate::generated::{Color, Item, ItemUnion, ItemVec, RawImage, URI};
use molecule::prelude::{Builder, Byte, Entity};
use serde_json::Value;
use types::{
    DOB0Output, DOB0TraitValue, Error, ImageType, Parameters, ParsedTrait, Pattern, TraitSchema,
};

macro_rules! item {
    ($itemty: ident, $value: ident) => {
        $itemty::new_builder()
            .set($value.as_bytes().iter().map(|v| Byte::new(*v)).collect())
            .build()
    };
}

pub fn dobs_parse_parameters(args: Vec<&[u8]>) -> Result<Parameters, Error> {
    if args.len() != 2 {
        return Err(Error::ParseInvalidArgCount);
    }

    let dob0_output: Vec<DOB0Output> = {
        let output = args[0];
        if output.is_empty() {
            return Err(Error::ParseInvalidDOB0Output);
        }
        serde_json::from_slice(output).map_err(|_| Error::ParseInvalidDOB0Output)?
    };
    let images_base = {
        let value = args[1];
        let traits_pool: Vec<Vec<Value>> =
            serde_json::from_slice(value).map_err(|_| Error::ParseInvalidTraitsBase)?;
        decode_trait_schema(traits_pool)?
    };
    Ok(Parameters {
        dob0_output,
        images_base,
    })
}

pub fn dobs_parse_syscall_parameters(
    parameters: &Parameters,
) -> Result<Vec<(String, ItemVec)>, Error> {
    let Parameters {
        dob0_output,
        images_base,
    } = parameters;

    let syscall_parameters = images_base
        .chunk_by(|a, b| a.name == b.name)
        .map(|images| {
            let mut items = ItemVec::new_builder();
            let mut name = String::new();
            for image in images.iter() {
                name.clone_from(&image.name); // names are the same
                let Some(value) = get_dob0_value_by_name(&image.dob0_trait, dob0_output) else {
                    break;
                };
                let value = match image.pattern {
                    Pattern::Options | Pattern::Range => {
                        let args = image.args.as_ref().ok_or(Error::DecodeInvalidOptionArgs)?;
                        get_dob1_value_by_dob0_value(args, value)?
                    }
                    Pattern::Raw => Some(
                        value
                            .get_string()
                            .cloned()
                            .map_err(|_| Error::DecodeInvalidRawValue)?,
                    ),
                };
                let Some(value) = value else {
                    break;
                };
                let item = match image.type_ {
                    ImageType::ColorCode => ItemUnion::from(item!(Color, value)),
                    ImageType::URI => ItemUnion::from(item!(URI, value)),
                    ImageType::RawImage => ItemUnion::from(item!(RawImage, value)),
                };
                items = items.push(Item::new_builder().set(item).build());
            }
            Ok((name, items.build()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(syscall_parameters)
}

pub(crate) fn decode_trait_schema(traits_pool: Vec<Vec<Value>>) -> Result<Vec<TraitSchema>, Error> {
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

fn get_dob0_value_by_name(trait_name: &str, dob0_output: &[DOB0Output]) -> Option<ParsedTrait> {
    dob0_output.iter().find_map(|output| {
        if output.name == trait_name {
            output.traits.first().cloned()
        } else {
            None
        }
    })
}

fn get_dob1_value_by_dob0_value(
    args: &BTreeMap<DOB0TraitValue, String>,
    dob0_value: ParsedTrait,
) -> Result<Option<String>, Error> {
    for (key, value) in args {
        match key {
            DOB0TraitValue::Number(number) => {
                let dob0_number = dob0_value.get_number()?;
                if dob0_number == *number {
                    return Ok(Some(value.clone()));
                }
            }
            DOB0TraitValue::String(string) => {
                let dob0_string = dob0_value.get_string()?;
                if dob0_string == string {
                    return Ok(Some(value.clone()));
                }
            }
            DOB0TraitValue::Range(start, end) => {
                let dob0_number = dob0_value.get_number()?;
                if *start <= dob0_number && dob0_number <= *end {
                    return Ok(Some(value.clone()));
                }
            }
            DOB0TraitValue::Any => return Ok(Some(value.clone())),
        }
    }
    Ok(None)
}
