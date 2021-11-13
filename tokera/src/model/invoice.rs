use serde::*;
use strum_macros::EnumIter;
use strum_macros::Display;
use ate::prelude::*;

use crate::model::*;

/// Represent a line item in an invoice that breaks down the
/// charges and what they are for.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InvoiceItem
{
    /// The name of the line item
    pub name: String,
    /// Amount of quantity that was consumed
    pub quantity: Decimal,
    /// Amount for this particular line item
    pub amount: Decimal,
    /// Any sales tax to be paid on this line item
    pub gst: Option<Decimal>,
    /// Amount plus the GST
    pub total: Decimal,
}

/// Determines the status of the invoice
#[derive(Serialize, Deserialize, Debug, Display, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, EnumIter)]
pub enum InvoiceStatus
{
    Unpaid,
    Paid,
}

/// Represents an invoice for some services that were charged
/// to a customer
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Invoice
{
    /// The current status of this invoice
    pub status: InvoiceStatus,
    /// Reference number of the contract this invoice is related to
    pub reference_number: String,
    /// What this invoice is related to
    pub related_to: String,
    /// Who paid the funds
    pub from_identity: String,
    /// Who received the funds
    pub to_identity: String,
    /// Wallet that the sender used
    pub from_wallet: PrimaryKey,
    /// The currency that the invoice will be paid in
    pub currency: NationalCurrency,
    /// The country you are resident in for tax purposes
    pub gst_country: Country,
    /// List of the line items that make up the invoice
    pub items: Vec<InvoiceItem>,

    /// Total amount from all the line item
    pub total_amount: Decimal,
    /// Total TAX to be paid for this line item
    pub total_gst: Option<Decimal>,
    /// Total amount paid for this invoice
    pub total_due: Decimal,
}