use crate::api::FileApi;
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Directory
{
}

impl FileApi
for Directory
{
}