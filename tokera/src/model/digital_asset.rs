use serde::*;

/// Digital assets are virtual things that have an intrinsic value to the humans
/// that interact with that particular digital environment/ecosystem.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DigitalAsset
{
    GameAccount,
    GameAsset,
    GameCurrency
}

impl DigitalAsset
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
            DigitalAsset::GameAccount => ("Game Account", "Access rights and ownership of a gaming account within a virtual world."),
            DigitalAsset::GameAsset => ("Game Item", "Tradable game asset within a unique virtual world that may hold some value in that realm."),
            DigitalAsset::GameCurrency => ("Game Current", "Internal currency within the virtual world of a particular game."),
        }
    }
}