pub struct WizardPrompt {
    pub prompt: String,
    pub echo: bool,
}

pub enum WizardAction {
    Challenge {
        name: String,
        instructions: String,
        prompts: Vec<WizardPrompt>,
    },
    Shell,
    Terminate {
        with_message: Option<String>,
    },
}

/// The wizard ABI allows for actions to be taken before the terminal
/// is actually fully running
#[async_trait::async_trait]
pub trait WizardAbi {
    async fn process(&mut self, responses: Vec<String>) -> WizardAction;

    fn token(&self) -> Option<String>;
}
