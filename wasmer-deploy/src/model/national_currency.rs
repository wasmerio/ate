use clap::Parser;
use serde::*;
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::Display;
use strum_macros::EnumIter;

/// Lists all the different national currencies that are available in the
/// world plus a set of attributes.
#[derive(
    Serialize,
    Deserialize,
    Display,
    Debug,
    Parser,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    EnumIter,
)]
pub enum NationalCurrency {
    #[clap()]
    NON,
    #[clap()]
    TOK,
    #[clap()]
    INV,
    #[clap()]
    BIT,
    #[clap()]
    EUR,
    #[clap()]
    USD,
    #[clap()]
    AUD,
    #[clap()]
    HKD,
    #[clap()]
    CNY,
    #[clap()]
    GBP,
    #[clap()]
    AAD,
    #[clap()]
    AFN,
    #[clap()]
    ALL,
    #[clap()]
    DZD,
    #[clap()]
    AOA,
    #[clap()]
    XCD,
    #[clap()]
    ARS,
    #[clap()]
    AMD,
    #[clap()]
    AWG,
    #[clap()]
    AZN,
    #[clap()]
    BSD,
    #[clap()]
    BHD,
    #[clap()]
    BDT,
    #[clap()]
    BBD,
    #[clap()]
    BYR,
    #[clap()]
    BZD,
    #[clap()]
    BMD,
    #[clap()]
    BTN,
    #[clap()]
    INR,
    #[clap()]
    BOB,
    #[clap()]
    BOV,
    #[clap()]
    BWP,
    #[clap()]
    NOK,
    #[clap()]
    BRL,
    #[clap()]
    BND,
    #[clap()]
    BGN,
    #[clap()]
    BIF,
    #[clap()]
    KHR,
    #[clap()]
    CAD,
    #[clap()]
    CVE,
    #[clap()]
    KYD,
    #[clap()]
    CLF,
    #[clap()]
    CLP,
    #[clap()]
    COP,
    #[clap()]
    COU,
    #[clap()]
    KMF,
    #[clap()]
    CDF,
    #[clap()]
    NZD,
    #[clap()]
    CRC,
    #[clap()]
    HRK,
    #[clap()]
    CUC,
    #[clap()]
    CUP,
    #[clap()]
    ANG,
    #[clap()]
    CZK,
    #[clap()]
    DKK,
    #[clap()]
    DJF,
    #[clap()]
    DOP,
    #[clap()]
    EGP,
    #[clap()]
    SVC,
    #[clap()]
    ERN,
    #[clap()]
    ETB,
    #[clap()]
    FKP,
    #[clap()]
    FJD,
    #[clap()]
    GMD,
    #[clap()]
    GEL,
    #[clap()]
    GHS,
    #[clap()]
    GIP,
    #[clap()]
    GTQ,
    #[clap()]
    GNF,
    #[clap()]
    GYD,
    #[clap()]
    HTG,
    #[clap()]
    HNL,
    #[clap()]
    HUF,
    #[clap()]
    ISK,
    #[clap()]
    IDR,
    #[clap()]
    IRR,
    #[clap()]
    IQD,
    #[clap()]
    ILS,
    #[clap()]
    JMD,
    #[clap()]
    JPY,
    #[clap()]
    JOD,
    #[clap()]
    KZT,
    #[clap()]
    KES,
    #[clap()]
    KPW,
    #[clap()]
    KRW,
    #[clap()]
    KWD,
    #[clap()]
    KGS,
    #[clap()]
    LAK,
    #[clap()]
    LBP,
    #[clap()]
    LSL,
    #[clap()]
    ZAR,
    #[clap()]
    LRD,
    #[clap()]
    LYD,
    #[clap()]
    CHF,
    #[clap()]
    LTL,
    #[clap()]
    MOP,
    #[clap()]
    MKD,
    #[clap()]
    MGA,
    #[clap()]
    MWK,
    #[clap()]
    MYR,
    #[clap()]
    MVR,
    #[clap()]
    MRO,
    #[clap()]
    MUR,
    #[clap()]
    MXN,
    #[clap()]
    MDL,
    #[clap()]
    MNT,
    #[clap()]
    MAD,
    #[clap()]
    MZN,
    #[clap()]
    MMK,
    #[clap()]
    NAD,
    #[clap()]
    NPR,
    #[clap()]
    NIO,
    #[clap()]
    NGN,
    #[clap()]
    OMR,
    #[clap()]
    PKR,
    #[clap()]
    PAB,
    #[clap()]
    PGK,
    #[clap()]
    PYG,
    #[clap()]
    PEN,
    #[clap()]
    PHP,
    #[clap()]
    PLN,
    #[clap()]
    QAR,
    #[clap()]
    RON,
    #[clap()]
    RUB,
    #[clap()]
    RWF,
    #[clap()]
    SHP,
    #[clap()]
    WST,
    #[clap()]
    STD,
    #[clap()]
    SAR,
    #[clap()]
    RSD,
    #[clap()]
    SCR,
    #[clap()]
    SLL,
    #[clap()]
    SGD,
    #[clap()]
    XSU,
    #[clap()]
    SBD,
    #[clap()]
    SOS,
    #[clap()]
    SSP,
    #[clap()]
    LKR,
    #[clap()]
    SDG,
    #[clap()]
    SRD,
    #[clap()]
    SZL,
    #[clap()]
    SEK,
    #[clap()]
    CHE,
    #[clap()]
    CHW,
    #[clap()]
    SYP,
    #[clap()]
    TWD,
    #[clap()]
    TJS,
    #[clap()]
    TZS,
    #[clap()]
    THB,
    #[clap()]
    TOP,
    #[clap()]
    TTD,
    #[clap()]
    TND,
    #[clap()]
    TRY,
    #[clap()]
    TMT,
    #[clap()]
    UGX,
    #[clap()]
    UAH,
    #[clap()]
    AED,
    #[clap()]
    UYU,
    #[clap()]
    UZS,
    #[clap()]
    VUV,
    #[clap()]
    VEF,
    #[clap()]
    VND,
    #[clap()]
    YER,
    #[clap()]
    XAF,
    #[clap()]
    XOF,
    #[clap()]
    BAM,
    #[clap()]
    ZMW,
    #[clap()]
    ZWL,
    #[clap()]
    XPF,
}

impl NationalCurrency {
    pub fn name(&self) -> &str {
        self.params().0
    }

    pub fn description(&self) -> String {
        format!("National currency of feign money ({})", self.name())
    }

    pub fn code(&self) -> &str {
        self.params().1
    }

    pub fn decimal_points(&self) -> i32 {
        self.params().2
    }

    /// Can this currency be used as a trade medium between good and services
    pub fn can_trade(&self) -> bool {
        self.params().3
    }

    /// Can this currency be exchanged for other currencies
    pub fn can_exchange(&self) -> bool {
        self.params().4
    }

    pub fn can_use_in_paypal(&self) -> bool {
        self.params().5
    }

    pub fn auto_convert(&self) -> bool {
        self.params().6
    }

    pub fn crypto_url(&self) -> Option<&str> {
        self.params().7
    }

    fn params(&self) -> (&str, &str, i32, bool, bool, bool, bool, Option<&str>) {
        match self {
            NationalCurrency::NON => (
                "Unknown currency",
                "000",
                0,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::TOK => (
                "Wasmer Coin",
                "001",
                0,
                true,
                true,
                false,
                true,
                Some("coin.wasmer.sh"),
            ),
            NationalCurrency::INV => (
                "Wasmer Invest",
                "002",
                0,
                false,
                true,
                false,
                false,
                Some("invest.wasmer.sh"),
            ),
            NationalCurrency::BIT => (
                "Bitcoin",
                "003",
                0,
                false,
                false,
                false,
                false,
                Some("bitcoin.exchange.wasmer.sh"),
            ),
            NationalCurrency::EUR => (
                "Euro",
                "978",
                2,
                false,
                true,
                true,
                false,
                Some("eur.exchange.wasmer.sh"),
            ),
            NationalCurrency::USD => (
                "US Dollar",
                "840",
                2,
                false,
                true,
                true,
                false,
                Some("usd.exchange.wasmer.sh"),
            ),
            NationalCurrency::AUD => (
                "Australian Dollar",
                "036",
                2,
                false,
                true,
                true,
                false,
                Some("aud.exchange.wasmer.sh"),
            ),
            NationalCurrency::HKD => (
                "Hong Kong Dollar",
                "344",
                2,
                false,
                true,
                true,
                false,
                Some("hkd.exchange.wasmer.sh"),
            ),
            NationalCurrency::CNY => (
                "Yuan Renminbi",
                "156",
                2,
                false,
                true,
                false,
                false,
                Some("cny.exchange.wasmer.sh"),
            ),
            NationalCurrency::GBP => (
                "Pound Sterling",
                "826",
                2,
                false,
                true,
                true,
                false,
                Some("gbp.exchange.wasmer.sh"),
            ),
            NationalCurrency::AAD => (
                "Antarctic Dollar",
                "004",
                2,
                false,
                false,
                true,
                false,
                None,
            ),
            NationalCurrency::AFN => ("Afghani", "971", 2, false, false, false, false, None),
            NationalCurrency::ALL => ("Lek", "008", 2, false, false, false, false, None),
            NationalCurrency::DZD => ("Algerian Dinar", "012", 2, false, false, false, false, None),
            NationalCurrency::AOA => ("Kwanza", "973", 2, false, false, false, false, None),
            NationalCurrency::XCD => (
                "East Caribbean Dollar",
                "951",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::ARS => ("Argentine Peso", "032", 2, false, false, false, false, None),
            NationalCurrency::AMD => ("Armenian Dram", "051", 2, false, false, false, false, None),
            NationalCurrency::AWG => ("Aruban Florin", "533", 2, false, false, false, false, None),
            NationalCurrency::AZN => (
                "Azerbaijanian Manat",
                "944",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::BSD => (
                "Bahamian Dollar",
                "044",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::BHD => ("Bahraini Dinar", "048", 3, false, false, false, false, None),
            NationalCurrency::BDT => ("Taka", "050", 2, false, false, false, false, None),
            NationalCurrency::BBD => (
                "Barbados Dollar",
                "052",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::BYR => (
                "Belarussian Ruble",
                "974",
                0,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::BZD => ("Belize Dollar", "084", 2, false, false, false, false, None),
            NationalCurrency::BMD => (
                "Bermudian Dollar",
                "060",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::BTN => ("Ngultrum", "064", 2, false, false, false, false, None),
            NationalCurrency::INR => ("Indian Rupee", "356", 2, false, false, false, false, None),
            NationalCurrency::BOB => ("Boliviano", "068", 2, false, false, false, false, None),
            NationalCurrency::BOV => ("Mvdol", "984", 2, false, false, false, false, None),
            NationalCurrency::BWP => ("Pula", "072", 2, false, false, false, false, None),
            NationalCurrency::NOK => (
                "Norwegian Krone",
                "578",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::BRL => ("Brazilian Real", "986", 2, false, false, false, false, None),
            NationalCurrency::BND => ("Brunei Dollar", "096", 2, false, false, false, false, None),
            NationalCurrency::BGN => ("Bulgarian Lev", "975", 2, false, false, false, false, None),
            NationalCurrency::BIF => ("Burundi Franc", "108", 0, false, false, false, false, None),
            NationalCurrency::KHR => ("Riel", "116", 2, false, false, false, false, None),
            NationalCurrency::CAD => (
                "Canadian Dollar",
                "124",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::CVE => (
                "Cape Verde Escudo",
                "132",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::KYD => (
                "Cayman Islands Dollar",
                "136",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::CLF => (
                "Unidad de Fomento",
                "990",
                4,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::CLP => ("Chilean Peso", "152", 0, false, false, false, false, None),
            NationalCurrency::COP => ("Colombian Peso", "170", 2, false, false, false, false, None),
            NationalCurrency::COU => (
                "Unidad de Valor Real",
                "970",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::KMF => ("Comoro Franc", "174", 0, false, false, false, false, None),
            NationalCurrency::CDF => (
                "Congolese Franc",
                "976",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::NZD => (
                "New Zealand Dollar",
                "554",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::CRC => (
                "Costa Rican Colon",
                "188",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::HRK => ("Croatian Kuna", "191", 2, false, false, false, false, None),
            NationalCurrency::CUC => (
                "Peso Convertible",
                "931",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::CUP => ("Cuban Peso", "192", 2, false, false, false, false, None),
            NationalCurrency::ANG => (
                "Netherlands Antillean Guilder",
                "532",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::CZK => ("Czech Koruna", "203", 2, false, false, false, false, None),
            NationalCurrency::DKK => ("Danish Krone", "208", 2, false, false, false, false, None),
            NationalCurrency::DJF => ("Djibouti Franc", "262", 0, false, false, false, false, None),
            NationalCurrency::DOP => ("Dominican Peso", "214", 2, false, false, false, false, None),
            NationalCurrency::EGP => ("Egyptian Pound", "818", 2, false, false, false, false, None),
            NationalCurrency::SVC => (
                "El Salvador Colon",
                "222",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::ERN => ("Nakfa", "232", 2, false, false, false, false, None),
            NationalCurrency::ETB => ("Ethiopian Birr", "230", 2, false, false, false, false, None),
            NationalCurrency::FKP => (
                "Falkland Islands Pound",
                "238",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::FJD => ("Fiji Dollar", "242", 2, false, false, false, false, None),
            NationalCurrency::GMD => ("Dalasi", "270", 2, false, false, false, false, None),
            NationalCurrency::GEL => ("Lari", "981", 2, false, false, false, false, None),
            NationalCurrency::GHS => ("Ghana Cedi", "936", 2, false, false, false, false, None),
            NationalCurrency::GIP => (
                "Gibraltar Pound",
                "292",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::GTQ => ("Quetzal", "320", 2, false, false, false, false, None),
            NationalCurrency::GNF => ("Guinea Franc", "324", 0, false, false, false, false, None),
            NationalCurrency::GYD => ("Guyana Dollar", "328", 2, false, false, false, false, None),
            NationalCurrency::HTG => ("Gourde", "332", 2, false, false, false, false, None),
            NationalCurrency::HNL => ("Lempira", "340", 2, false, false, false, false, None),
            NationalCurrency::HUF => ("Forint", "348", 2, false, false, false, false, None),
            NationalCurrency::ISK => ("Iceland Krona", "352", 0, false, false, false, false, None),
            NationalCurrency::IDR => ("Rupiah", "360", 2, false, false, false, false, None),
            NationalCurrency::IRR => ("Iranian Rial", "364", 2, false, false, false, false, None),
            NationalCurrency::IQD => ("Iraqi Dinar", "368", 3, false, false, false, false, None),
            NationalCurrency::ILS => (
                "New Israeli Sheqel",
                "376",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::JMD => (
                "Jamaican Dollar",
                "388",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::JPY => ("Yen", "392", 0, false, false, false, false, None),
            NationalCurrency::JOD => (
                "Jordanian Dinar",
                "400",
                3,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::KZT => ("Tenge", "398", 2, false, false, false, false, None),
            NationalCurrency::KES => (
                "Kenyan Shilling",
                "404",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::KPW => (
                "North Korean Won",
                "408",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::KRW => ("Won", "410", 0, false, false, false, false, None),
            NationalCurrency::KWD => ("Kuwaiti Dinar", "414", 3, false, false, false, false, None),
            NationalCurrency::KGS => ("Som", "417", 2, false, false, false, false, None),
            NationalCurrency::LAK => ("Kip", "418", 2, false, false, false, false, None),
            NationalCurrency::LBP => ("Lebanese Pound", "422", 2, false, false, false, false, None),
            NationalCurrency::LSL => ("Loti", "426", 2, false, false, false, false, None),
            NationalCurrency::ZAR => ("Rand", "710", 2, false, false, false, false, None),
            NationalCurrency::LRD => (
                "Liberian Dollar",
                "430",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::LYD => ("Libyan Dinar", "434", 3, false, false, false, false, None),
            NationalCurrency::CHF => ("Swiss Franc", "756", 2, false, false, false, false, None),
            NationalCurrency::LTL => (
                "Lithuanian Litas",
                "440",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::MOP => ("Pataca", "446", 2, false, false, false, false, None),
            NationalCurrency::MKD => ("Denar", "807", 2, false, false, false, false, None),
            NationalCurrency::MGA => (
                "Malagasy Ariary",
                "969",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::MWK => ("Kwacha", "454", 2, false, false, false, false, None),
            NationalCurrency::MYR => (
                "Malaysian Ringgit",
                "458",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::MVR => ("Rufiyaa", "462", 2, false, false, false, false, None),
            NationalCurrency::MRO => ("Ouguiya", "478", 2, false, false, false, false, None),
            NationalCurrency::MUR => (
                "Mauritius Rupee",
                "480",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::MXN => ("Mexican Peso", "484", 2, false, false, false, false, None),
            NationalCurrency::MDL => ("Moldovan Leu", "498", 2, false, false, false, false, None),
            NationalCurrency::MNT => ("Tugrik", "496", 2, false, false, false, false, None),
            NationalCurrency::MAD => (
                "Moroccan Dirham",
                "504",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::MZN => (
                "Mozambique Metical",
                "943",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::MMK => ("Kyat", "104", 2, false, false, false, false, None),
            NationalCurrency::NAD => ("Namibia Dollar", "516", 2, false, false, false, false, None),
            NationalCurrency::NPR => ("Nepalese Rupee", "524", 2, false, false, false, false, None),
            NationalCurrency::NIO => ("Cordoba Oro", "558", 2, false, false, false, false, None),
            NationalCurrency::NGN => ("Naira", "566", 2, false, false, false, false, None),
            NationalCurrency::OMR => ("Rial Omani", "512", 3, false, false, false, false, None),
            NationalCurrency::PKR => ("Pakistan Rupee", "586", 2, false, false, false, false, None),
            NationalCurrency::PAB => ("Balboa", "590", 2, false, false, false, false, None),
            NationalCurrency::PGK => ("Kina", "598", 2, false, false, false, false, None),
            NationalCurrency::PYG => ("Guarani", "600", 0, false, false, false, false, None),
            NationalCurrency::PEN => ("Nuevo Sol", "604", 2, false, false, false, false, None),
            NationalCurrency::PHP => (
                "Philippine Peso",
                "608",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::PLN => ("Zloty", "985", 2, false, false, false, false, None),
            NationalCurrency::QAR => ("Qatari Rial", "634", 2, false, false, false, false, None),
            NationalCurrency::RON => (
                "New Romanian Leu",
                "946",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::RUB => ("Russian Ruble", "643", 2, false, false, false, false, None),
            NationalCurrency::RWF => ("Rwanda Franc", "646", 0, false, false, false, false, None),
            NationalCurrency::SHP => (
                "Saint Helena Pound",
                "654",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::WST => ("Tala", "882", 2, false, false, false, false, None),
            NationalCurrency::STD => ("Dobra", "678", 2, false, false, false, false, None),
            NationalCurrency::SAR => ("Saudi Riyal", "682", 2, false, false, false, false, None),
            NationalCurrency::RSD => ("Serbian Dinar", "941", 2, false, false, false, false, None),
            NationalCurrency::SCR => (
                "Seychelles Rupee",
                "690",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::SLL => ("Leone", "694", 2, false, false, false, false, None),
            NationalCurrency::SGD => (
                "Singapore Dollar",
                "702",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::XSU => ("Sucre", "994", 0, false, false, false, false, None),
            NationalCurrency::SBD => (
                "Solomon Islands Dollar",
                "090",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::SOS => (
                "Somali Shilling",
                "706",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::SSP => (
                "South Sudanese Pound",
                "728",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::LKR => (
                "Sri Lanka Rupee",
                "144",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::SDG => ("Sudanese Pound", "938", 2, false, false, false, false, None),
            NationalCurrency::SRD => ("Surinam Dollar", "968", 2, false, false, false, false, None),
            NationalCurrency::SZL => ("Lilangeni", "748", 2, false, false, false, false, None),
            NationalCurrency::SEK => ("Swedish Krona", "752", 2, false, false, false, false, None),
            NationalCurrency::CHE => ("WIR Euro", "947", 2, false, false, false, false, None),
            NationalCurrency::CHW => ("WIR Franc", "948", 2, false, false, false, false, None),
            NationalCurrency::SYP => ("Syrian Pound", "760", 2, false, false, false, false, None),
            NationalCurrency::TWD => (
                "New Taiwan Dollar",
                "901",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::TJS => ("Somoni", "972", 2, false, false, false, false, None),
            NationalCurrency::TZS => (
                "Tanzanian Shilling",
                "834",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::THB => ("Baht", "764", 2, false, false, false, false, None),
            NationalCurrency::TOP => ("Pa’anga", "776", 2, false, false, false, false, None),
            NationalCurrency::TTD => (
                "Trinidad and Tobago Dollar",
                "780",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::TND => ("Tunisian Dinar", "788", 3, false, false, false, false, None),
            NationalCurrency::TRY => ("Turkish Lira", "949", 2, false, false, false, false, None),
            NationalCurrency::TMT => (
                "Turkmenistan New Manat",
                "934",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::UGX => (
                "Uganda Shilling",
                "800",
                0,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::UAH => ("Hryvnia", "980", 2, false, false, false, false, None),
            NationalCurrency::AED => ("UAE Dirham", "784", 2, false, false, false, false, None),
            NationalCurrency::UYU => ("Peso Uruguayo", "858", 2, false, false, false, false, None),
            NationalCurrency::UZS => ("Uzbekistan Sum", "860", 2, false, false, false, false, None),
            NationalCurrency::VUV => ("Vatu", "548", 0, false, false, false, false, None),
            NationalCurrency::VEF => ("Bolivar", "937", 2, false, false, false, false, None),
            NationalCurrency::VND => ("Dong", "704", 0, false, false, false, false, None),
            NationalCurrency::YER => ("Yemeni Rial", "886", 2, false, false, false, false, None),
            NationalCurrency::XAF => (
                "Central African CFA franc",
                "950",
                0,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::XOF => (
                "West African CFA franc",
                "952",
                0,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::BAM => (
                "Bosnia and Herzegovina convertible mark",
                "977",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::ZMW => ("Zambian Kwacha", "967", 2, false, false, false, false, None),
            NationalCurrency::ZWL => (
                "Zimbabwe Dollar",
                "932",
                2,
                false,
                false,
                false,
                false,
                None,
            ),
            NationalCurrency::XPF => ("CFP franc", "953", 0, false, false, false, false, None),
        }
    }
}

impl FromStr for NationalCurrency {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for currency in NationalCurrency::iter() {
            if currency.to_string().eq_ignore_ascii_case(s) {
                return Ok(currency);
            }
            if currency.name().eq_ignore_ascii_case(s) {
                return Ok(currency);
            }
            if currency.code().eq_ignore_ascii_case(s) {
                return Ok(currency);
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Currency is not valid: {}.", s),
        ))
    }
}

impl Default for NationalCurrency {
    fn default() -> Self {
        Self::EUR
    }
}
