#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::error::Error;

use super::*;

#[derive(Debug, Default)]
pub struct ProcessError
{
    pub sink_errors: Vec<SinkError>,
    pub validation_errors: Vec<ValidationError>,
}

impl ProcessError {
    pub fn has_errors(&self) -> bool {
        if self.sink_errors.is_empty() == false { return true; }
        if self.validation_errors.is_empty() == false { return true; }
        false
    }

    pub fn as_result(self) -> Result<(), ProcessError> {
        match self.has_errors() {
            true => Err(self),
            false => Ok(())
        }
    }
}

impl std::fmt::Display
for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut err = "Processing error - ".to_string();
        for sink in self.sink_errors.iter() {
            err = err + &sink.to_string()[..] + " - ";
        }
        for validation in self.validation_errors.iter() {
            err = err + &validation.to_string()[..] + " - ";
        }
        write!(f, "{}", err)
    }
}

impl std::error::Error
for ProcessError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}