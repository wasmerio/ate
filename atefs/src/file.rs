use crate::api::FileApi;
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegularFile
{
}

impl FileApi
for RegularFile
{
}