use std::sync::Arc;

use crate::api::ConsoleAbi;
use crate::api::WizardAbi;
use crate::api::WizardAction;
use crate::api::WizardPrompt;

pub struct WizardExecutor {
    wizard: Box<dyn WizardAbi + Send + Sync + 'static>,
    prompts: Vec<WizardPrompt>,
    responses: Vec<String>,
}

pub enum WizardExecutorAction {
    More {
        echo: bool
    },
    Done
}

impl WizardExecutor {
    pub fn new(wizard: Box<dyn WizardAbi + Send + Sync + 'static>) -> Self {
        Self {
            wizard,
            prompts: Vec::new(),
            responses: Vec::new(),
        }
    }

    pub fn token(&self) -> Option<String> {
        self.wizard.token()
    }

    pub async fn feed(&mut self, abi: &Arc<dyn ConsoleAbi>, data: Option<String>) -> WizardExecutorAction {
        if let Some(data) = data {
            self.responses.push(data);
            abi.stdout("\r\n".to_string().into_bytes()).await;
        }
        
        if let Some(prompt) = self.prompts.pop() {
            abi.stdout(text_to_bytes(prompt.prompt)).await;
            WizardExecutorAction::More {
                echo: prompt.echo
            }
        } else {
            let responses = self.responses.drain(..).collect();
            match self.wizard.process(responses).await {
                WizardAction::Challenge { name, instructions, mut prompts, } => {
                    if name.len() > 0 {
                        abi.stdout(text_to_bytes(name)).await;
                        abi.stdout("\r\n".to_string().into_bytes()).await;
                    }
                    if instructions.len() > 0 {
                        abi.stdout(text_to_bytes(instructions)).await;
                        abi.stdout("\r\n".to_string().into_bytes()).await;
                    }
    
                    prompts.reverse();
                    self.prompts.append(&mut prompts);
    
                    if let Some(prompt) = self.prompts.pop() {
                        abi.stdout(text_to_bytes(prompt.prompt)).await;
                        WizardExecutorAction::More {
                            echo: prompt.echo
                        }
                    } else {
                        WizardExecutorAction::More { echo: false }
                    }
                },
                WizardAction::Shell => {
                    WizardExecutorAction::Done
                },
                WizardAction::Terminate { with_message } => {
                    if let Some(msg) = with_message { 
                        abi.stdout(text_to_bytes(msg)).await;
                        abi.stdout("\r\n".to_string().into_bytes()).await;
                    }
                    abi.exit().await;
                    WizardExecutorAction::More { echo: false }
                }
            }
        }
    }
}

fn text_to_bytes(txt: String) -> Vec<u8>
{
    txt.replace("\\", "\\\\").replace("\n", "\r\n").into_bytes()
}