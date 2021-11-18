
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct ExitStatusError
{
    code: Option<i32>,
}

impl ExitStatusError {
    pub fn code(&self) -> Option<i32> {
        self.code.clone()
    }
    
    pub fn into_status(&self) -> ExitStatus {
        ExitStatus {
            code: self.code
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct ExitStatus
{
    pub(super) code: Option<i32>,
}

impl ExitStatus
{
    pub fn exit_ok(&self) -> Result<(), ExitStatusError> {
        match self.code {
            Some(a) if a == 0 => {
                return Ok(())
            },
            Some(a) => {
                return Err(ExitStatusError { code: Some(a) });
            },
            None => {
                return Ok(())
            }
        }
    }

    pub fn success(&self) -> bool {
        self.code.unwrap_or(0) == 0
    }

    pub fn code(&self) -> Option<i32> {
        self.code
    }
}