use alloc::{collections::BTreeMap, string::String, vec::Vec};
use serde_json::Value;

use crate::decoder::{
    decode_trait_schema, dobs_parse_parameters, dobs_parse_syscall_parameters,
    types::{DOB0TraitValue, ImageType, Pattern, TraitSchema},
};

impl TraitSchema {
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

#[test]
fn test_parse_syscall_parameters() {
    // generated from `test_generate_basic_example` case
    let dob0_output = "[{\"name\":\"Name\",\"traits\":[{\"String\":\"Ethan\"}]},{\"name\":\"Age\",\"traits\":[{\"Number\":23}]},{\"name\":\"Score\",\"traits\":[{\"Number\":136}]},{\"name\":\"DNA\",\"traits\":[{\"String\":\"0xaabbcc\"}]},{\"name\":\"URL\",\"traits\":[{\"String\":\"http://127.0.0.1:8090\"}]},{\"name\":\"Value\",\"traits\":[{\"Number\":13417386}]}]";
    let images_base = "[[\"0\",\"color\",\"Name\",\"options\",[[\"Alice\",\"#0000FF\"],[\"Bob\",\"#00FF00\"],[\"Ethan\",\"#FF0000\"],[[\"*\"],\"#FFFFFF\"]]],[\"0\",\"uri\",\"Age\",\"range\",[[[0,50],\"btcfs://b2f4560f17679d3e3fca66209ac425c660d28a252ef72444c3325c6eb0364393i0\"],[[51,100],\"btcfs://eb3910b3e32a5ed9460bd0d75168c01ba1b8f00cc0faf83e4d8b67b48ea79676i0\"],[[\"*\"],\"btcfs://11b6303eb7d887d7ade459ac27959754cd55f9f9e50345ced8e1e8f47f4581fai0\"]]],[\"0\",\"uri\",\"Score\",\"range\",[[[0,1000],\"btcfs://11d6cc654f4c0759bfee520966937a4304db2b33880c88c2a6c649e30c7b9aaei0\"],[[\"*\"],\"btcfs://e1484915b27e45b120239080fe5032580550ff9ff759eb26ee86bf8aaf90068bi0\"]]],[\"1\",\"uri\",\"Value\",\"range\",[[[0,100000],\"btcfs://11d6cc654f4c0759bfee520966937a4304db2b33880c88c2a6c649e30c7b9aaei0\"],[[\"*\"],\"btcfs://e1484915b27e45b120239080fe5032580550ff9ff759eb26ee86bf8aaf90068bi0\"]]]]";

    let args = vec![dob0_output.as_bytes(), images_base.as_bytes()];
    let parameters = dobs_parse_parameters(args).expect("parse parameters failed");
    let syscall_parameters =
        dobs_parse_syscall_parameters(&parameters).expect("parse syscall parameters failed");
    println!("{:?}", syscall_parameters);
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
