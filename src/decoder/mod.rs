use alloc::{collections::BTreeMap, string::String, vec::Vec};

pub mod types;
use crate::generated::{Color, Item, ItemUnion, ItemVec, RawImage, URI};
use molecule::prelude::{Builder, Byte, Entity};
use serde_json::Value;
use types::{
    decode_trait_schema, DOB0Output, DOB0TraitValue, Error, ImageType, Parameters, ParsedTrait,
    Pattern,
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

#[test]
fn test_parse_syscall_parameters() {
    // generated from `test_generate_basic_example` case
    let dob0_output = "[{\"name\":\"Name\",\"traits\":[{\"String\":\"Ethan\"}]},{\"name\":\"Age\",\"traits\":[{\"Number\":23}]},{\"name\":\"Score\",\"traits\":[{\"Number\":136}]},{\"name\":\"DNA\",\"traits\":[{\"String\":\"0xaabbcc\"}]},{\"name\":\"URL\",\"traits\":[{\"String\":\"http://127.0.0.1:8090\"}]},{\"name\":\"Value\",\"traits\":[{\"Number\":13417386}]}]";
    let images_base = "[[\"0\",\"color\",\"Name\",\"options\",[[\"Alice\",\"#0000FF\"],[\"Bob\",\"#00FF00\"],[\"Ethan\",\"#FF0000\"],[[\"*\"],\"#FFFFFF\"]]],[\"0\",\"uri\",\"Age\",\"range\",[[[0,50],\"btcfs://64f562d16e2a4a29e8c4821370fff473edfa22c26ef5808adb2404e39dc013e5i0\"],[[51,100],\"btcfs://c29fecd6d7d7eec0cb3a2b3dfdcb6aa26081db8f9851110b7c20a0f3c617299ai0\"],[[\"*\"],\"btcfs://a3589ddcf4b7a3c6da52fe6ae4ed3296f1ede139fe9127f2697ce0dcf2703b61i0\"]]],[\"1\",\"uri\",\"Score\",\"range\",[[[0,1000],\"btcfs://ba8b1bb9d8baee4bf24a06faa25b569410f2db96b4639f8e08ccbec05c88d79bi0\"],[[\"*\"],\"btcfs://b84ec0c770aa1961a3d9498ea8a67e1282532913fc1c13e3eaf5a48de2164fb9i0\"]]]]";

    let args = vec![dob0_output.as_bytes(), images_base.as_bytes()];
    let parameters = dobs_parse_parameters(args).expect("parse parameters failed");
    let syscall_parameters =
        dobs_parse_syscall_parameters(&parameters).expect("parse syscall parameters failed");
    println!("{:?}", syscall_parameters);
}
