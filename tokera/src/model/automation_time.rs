use serde::*;

/// Automation time is a safe and secure way to automate grinding activities
/// in a commercial way to drive revenue for digital creators.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AutomationTime
{
    ExclusiveBot,
}

impl AutomationTime
{
    pub fn name(&self) -> &str
    {
        self.params().0
    }

    pub fn description(&self) -> &str
    {
        self.params().1
    }

    fn params(&self) -> (&str, &str)
    {
        match self {
            AutomationTime::ExclusiveBot => ("Exclusive Bot", "Rental of a legal bot that will automate a particular task."),
        }
    }
}