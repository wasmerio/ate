#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::{Serialize, Deserialize};
use serde::{Serializer, de::Deserializer};

pub fn vec_serialize<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer
{
    let type_name = std::any::type_name::<S>();
    if type_name.contains("bincode::") || type_name.contains("rmp_serde") {
        serializer.serialize_bytes(&data[..])
    } else {
        serializer.serialize_str(&base64::encode(&data[..]))
    }
}

pub fn vec_deserialize<'a, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where D: Deserializer<'a>
{
    let type_name = std::any::type_name::<D>();
    if type_name.contains("bincode::") || type_name.contains("rmp_serde") {
        let ret = <&[u8]>::deserialize(deserializer)?;
        Ok(Vec::from(ret))
    } else {
        use serde::de::Error;
        let ret = String::deserialize(deserializer)
            .and_then(|string| base64::decode(&string)
            .map_err(|err| Error::custom(err.to_string()))
        )?;
        Ok(ret)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct TestClass
{
    pub header: String,
    #[serde(serialize_with = "vec_serialize", deserialize_with = "vec_deserialize")]
    pub my_bytes: Vec<u8>,
}

#[test]
fn test_b64() {
    crate::utils::bootstrap_test_env();

    let plain = TestClass {
        header: "ate".to_string(),
        my_bytes: vec![ 112u8, 84u8, 99u8, 210u8, 55u8, 201u8, 202u8, 203u8, 204u8, 205u8, 206u8, 207u8 ],
    };

    let cipher = bincode::serialize(&plain).unwrap();
    trace!("{:?}", cipher);
    let test: TestClass = bincode::deserialize(&cipher[..]).unwrap();
    assert_eq!(test, plain);

    let cipher = rmp_serde::to_vec(&plain).unwrap();
    trace!("{:?}", cipher);
    let test: TestClass = rmp_serde::from_read_ref(&cipher[..]).unwrap();
    assert_eq!(test, plain);

    let cipher = serde_json::to_string_pretty(&plain).unwrap();
    trace!("{}", cipher);
    let test: TestClass = serde_json::from_str(&cipher).unwrap();
    assert_eq!(test, plain);
}