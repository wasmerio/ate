use serde::*;
use std::str::FromStr;
use clap::Parser;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use strum_macros::Display;

use crate::model::*;

/// Lists all the different national currencies that are available in the
/// world plus a set of attributes.
#[derive(Serialize, Deserialize, Display, Debug, Parser, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter)]
pub enum Country
{
    #[clap()]
    ABW,
    #[clap()]
    AFG,
    #[clap()]
    AGO,
    #[clap()]
    AIA,
    #[clap()]
    ALA,
    #[clap()]
    ALB,
    #[clap()]
    AND,
    #[clap()]
    ARE,
    #[clap()]
    ARG,
    #[clap()]
    ARM,
    #[clap()]
    ASM,
    #[clap()]
    ATA,
    #[clap()]
    ATF,
    #[clap()]
    ATG,
    #[clap()]
    AUS,
    #[clap()]
    AUT,
    #[clap()]
    AZE,
    #[clap()]
    BDI,
    #[clap()]
    BEL,
    #[clap()]
    BEN,
    #[clap()]
    BES,
    #[clap()]
    BFA,
    #[clap()]
    BGD,
    #[clap()]
    BGR,
    #[clap()]
    BHR,
    #[clap()]
    BHS,
    #[clap()]
    BIH,
    #[clap()]
    BLM,
    #[clap()]
    BLR,
    #[clap()]
    BLZ,
    #[clap()]
    BMU,
    #[clap()]
    BOL,
    #[clap()]
    BRA,
    #[clap()]
    BRB,
    #[clap()]
    BRN,
    #[clap()]
    BTN,
    #[clap()]
    BVT,
    #[clap()]
    BWA,
    #[clap()]
    CAF,
    #[clap()]
    CAN,
    #[clap()]
    CCK,
    #[clap()]
    CHE,
    #[clap()]
    CHL,
    #[clap()]
    CHN,
    #[clap()]
    CIV,
    #[clap()]
    CMR,
    #[clap()]
    COD,
    #[clap()]
    COG,
    #[clap()]
    COK,
    #[clap()]
    COL,
    #[clap()]
    COM,
    #[clap()]
    CPV,
    #[clap()]
    CPI,
    #[clap()]
    CUB,
    #[clap()]
    CUW,
    #[clap()]
    CXR,
    #[clap()]
    CYM,
    #[clap()]
    CYP,
    #[clap()]
    CZE,
    #[clap()]
    DEU,
    #[clap()]
    DJI,
    #[clap()]
    DMA,
    #[clap()]
    DNK,
    #[clap()]
    DOM,
    #[clap()]
    DZA,
    #[clap()]
    ECU,
    #[clap()]
    EGY,
    #[clap()]
    ERI,
    #[clap()]
    ESP,
    #[clap()]
    EST,
    #[clap()]
    ETH,
    #[clap()]
    FIN,
    #[clap()]
    FJI,
    #[clap()]
    FLK,
    #[clap()]
    FRA,
    #[clap()]
    FRO,
    #[clap()]
    FSM,
    #[clap()]
    GAB,
    #[clap()]
    GBR,
    #[clap()]
    GEO,
    #[clap()]
    GGY,
    #[clap()]
    GHA,
    #[clap()]
    GIB,
    #[clap()]
    GIN,
    #[clap()]
    GLP,
    #[clap()]
    GMB,
    #[clap()]
    GNB,
    #[clap()]
    GNQ,
    #[clap()]
    GRC,
    #[clap()]
    GRD,
    #[clap()]
    GRL,
    #[clap()]
    GTM,
    #[clap()]
    GUF,
    #[clap()]
    GUM,
    #[clap()]
    GUY,
    #[clap()]
    HKG,
    #[clap()]
    HMD,
    #[clap()]
    HND,
    #[clap()]
    HRV,
    #[clap()]
    HTI,
    #[clap()]
    HUN,
    #[clap()]
    IDN,
    #[clap()]
    IMN,
    #[clap()]
    IND,
    #[clap()]
    IOT,
    #[clap()]
    IRL,
    #[clap()]
    IRN,
    #[clap()]
    IRQ,
    #[clap()]
    ISL,
    #[clap()]
    ISR,
    #[clap()]
    ITA,
    #[clap()]
    JAM,
    #[clap()]
    JEY,
    #[clap()]
    JOR,
    #[clap()]
    JPN,
    #[clap()]
    KAZ,
    #[clap()]
    KEN,
    #[clap()]
    KGZ,
    #[clap()]
    KHM,
    #[clap()]
    KIR,
    #[clap()]
    KNA,
    #[clap()]
    KOR,
    #[clap()]
    KWT,
    #[clap()]
    LAO,
    #[clap()]
    LBN,
    #[clap()]
    LBR,
    #[clap()]
    LBY,
    #[clap()]
    LCA,
    #[clap()]
    LIE,
    #[clap()]
    LKA,
    #[clap()]
    LSO,
    #[clap()]
    LTU,
    #[clap()]
    LUX,
    #[clap()]
    LVA,
    #[clap()]
    MAC,
    #[clap()]
    MAF,
    #[clap()]
    MAR,
    #[clap()]
    MCO,
    #[clap()]
    MDA,
    #[clap()]
    MDG,
    #[clap()]
    MDV,
    #[clap()]
    MEX,
    #[clap()]
    MHL,
    #[clap()]
    MKD,
    #[clap()]
    MLI,
    #[clap()]
    MLT,
    #[clap()]
    MMR,
    #[clap()]
    MNE,
    #[clap()]
    MNG,
    #[clap()]
    MNP,
    #[clap()]
    MOZ,
    #[clap()]
    MRT,
    #[clap()]
    MSR,
    #[clap()]
    MTQ,
    #[clap()]
    MUS,
    #[clap()]
    MWI,
    #[clap()]
    MYS,
    #[clap()]
    MYT,
    #[clap()]
    NAM,
    #[clap()]
    NCL,
    #[clap()]
    NER,
    #[clap()]
    NFK,
    #[clap()]
    NGA,
    #[clap()]
    NIC,
    #[clap()]
    NIU,
    #[clap()]
    NLD,
    #[clap()]
    NOR,
    #[clap()]
    NPL,
    #[clap()]
    NRU,
    #[clap()]
    NZL,
    #[clap()]
    OMN,
    #[clap()]
    PAK,
    #[clap()]
    PAN,
    #[clap()]
    PCN,
    #[clap()]
    PER,
    #[clap()]
    PHL,
    #[clap()]
    PLW,
    #[clap()]
    PNG,
    #[clap()]
    POL,
    #[clap()]
    PRI,
    #[clap()]
    PRK,
    #[clap()]
    PRT,
    #[clap()]
    PRY,
    #[clap()]
    PSE,
    #[clap()]
    PYF,
    #[clap()]
    QAT,
    #[clap()]
    REU,
    #[clap()]
    ROU,
    #[clap()]
    RUS,
    #[clap()]
    RWA,
    #[clap()]
    SAU,
    #[clap()]
    SDN,
    #[clap()]
    SEN,
    #[clap()]
    SGP,
    #[clap()]
    SGS,
    #[clap()]
    SHN,
    #[clap()]
    SJM,
    #[clap()]
    SLB,
    #[clap()]
    SLE,
    #[clap()]
    SLV,
    #[clap()]
    SMR,
    #[clap()]
    SOM,
    #[clap()]
    SPM,
    #[clap()]
    SRB,
    #[clap()]
    SSD,
    #[clap()]
    STP,
    #[clap()]
    SUR,
    #[clap()]
    SVK,
    #[clap()]
    SVN,
    #[clap()]
    SWE,
    #[clap()]
    SWZ,
    #[clap()]
    SXM,
    #[clap()]
    SYC,
    #[clap()]
    SYR,
    #[clap()]
    TCA,
    #[clap()]
    TCD,
    #[clap()]
    TGO,
    #[clap()]
    THA,
    #[clap()]
    TJK,
    #[clap()]
    TKL,
    #[clap()]
    TKM,
    #[clap()]
    TLS,
    #[clap()]
    TON,
    #[clap()]
    TTO,
    #[clap()]
    TUN,
    #[clap()]
    TUR,
    #[clap()]
    TUV,
    #[clap()]
    TWN,
    #[clap()]
    TZA,
    #[clap()]
    UGA,
    #[clap()]
    UKR,
    #[clap()]
    UMI,
    #[clap()]
    URY,
    #[clap()]
    USA,
    #[clap()]
    UZB,
    #[clap()]
    VAT,
    #[clap()]
    VCT,
    #[clap()]
    VEN,
    #[clap()]
    VGB,
    #[clap()]
    VIR,
    #[clap()]
    VNM,
    #[clap()]
    VUT,
    #[clap()]
    WLF,
    #[clap()]
    WSM,
    #[clap()]
    YEM,
    #[clap()]
    ZAF,
    #[clap()]
    ZMB,
    #[clap()]
    ZWE
}

impl Country
{    
    pub fn official_name(&self) -> &str {
        self.params().0

    }

    pub fn short_name(&self) -> &str {
        self.params().1
    }

    pub fn iso2(&self) -> &str {
        self.params().2
    }

    pub fn iso3(&self) -> &str {
        self.params().3
    }

    pub fn num3(&self) -> u16 {
        self.params().4
    }

    pub fn national_currency(&self) -> NationalCurrency {
        self.params().5
    }

    pub fn digital_gst(&self) -> Option<Decimal> {
        self.params().6
    }

    fn params(&self) -> (&str, &str, &str, &str, u16, NationalCurrency, Option<Decimal>)
    {
        match self {
            Country::ABW => (
                "Aruba",
                "Aruba",
                "AW",
                "ABW",
                533,
                NationalCurrency::AWG,
                None
            ),
            Country::AFG => (
                "Islamic Republic of Afghanistan",
                "Afghanistan",
                "AF",
                "AFG",
                004,
                NationalCurrency::AFN,
                None
            ),
            Country::AGO => (
                "Republic of Angola",
                "Angola",
                "AO",
                "AGO",
                024,
                NationalCurrency::AOA,
                Some(Decimal::from_str("14.0").unwrap())
            ),
            Country::AIA => (
                "Anguilla",
                "Anguilla",
                "AI",
                "AIA",
                660,
                NationalCurrency::XCD,
                None
            ),
            Country::ALA => (
                "Åland Islands",
                "Åland Islands",
                "AX",
                "ALA",
                248,
                NationalCurrency::EUR,
                None
            ),
            Country::ALB => (
                "Republic of Albania",
                "Albania",
                "AL",
                "ALB",
                008,
                NationalCurrency::ALL,
                Some(Decimal::from_str("20.0").unwrap())
            ),
            Country::AND => (
                "Principality of Andorra",
                "Andorra",
                "AN",
                "AND",
                020,
                NationalCurrency::EUR,
                Some(Decimal::from_str("4.5").unwrap())
            ),
            Country::ARE => (
                "United Arab Emirates",
                "Emirates",
                "AE",
                "ARE",
                784,
                NationalCurrency::AED,
                None
            ),
            Country::ARG => (
                "Argentine Republic",
                "Argentina",
                "AR",
                "ARG",
                032,
                NationalCurrency::ARS,
                Some(Decimal::from_str("20.0").unwrap())
            ),
            Country::ARM => (
                "Republic of Armenia",
                "Armenia",
                "AM",
                "ARM",
                051,
                NationalCurrency::AMD,
                Some(Decimal::from_str("20.0").unwrap())
            ),
            Country::ASM => (
                "American Samoa",
                "American Samoa",
                "AS",
                "ASM",
                016,
                NationalCurrency::WST,
                None
            ),
            Country::ATA => (
                "Antarctica",
                "Antarctica",
                "AQ",
                "ATA",
                010,
                NationalCurrency::AAD,
                None
            ),
            Country::ATF => (
                "French Southern and Antarctic Lands",
                "French Southern Territories",
                "TF",
                "ATF",
                260,
                NationalCurrency::EUR,
                None
            ),
            Country::ATG => (
                "Antigua and Barbuda",
                "Antigua and Barbuda",
                "AG",
                "ATG",
                028,
                NationalCurrency::XCD,
                None
            ),
            Country::AUS => (
                "Commonwealth of Australia",
                "Australia",
                "AU",
                "AUS",
                036,
                NationalCurrency::AUD,
                Some(Decimal::from_str("10.0").unwrap())
            ),
            Country::AUT => (
                "Republic of Austria",
                "Austria",
                "AT",
                "AUT",
                040,
                NationalCurrency::EUR,
                Some(Decimal::from_str("20.0").unwrap())
            ),
            Country::AZE => (
                "Republic of Azerbaijan",
                "Azerbaijan",
                "AZ",
                "AZE",
                031,
                NationalCurrency::AZN,
                Some(Decimal::from_str("12.0").unwrap())
            ),
            Country::BDI => (
                "Republic of Burundi",
                "Burundi",
                "BI",
                "BDI",
                108,
                NationalCurrency::BIF,
                None
            ),
            Country::BEL => (
                "Kingdom of Belgium",
                "Belgium",
                "BE",
                "BEL",
                056,
                NationalCurrency::EUR,
                Some(Decimal::from_str("21.0").unwrap())
            ),
            Country::BEN => ( 
                "Republic of Benin",
                "Benin",
                "BJ",
                "BEN",
                204,
                NationalCurrency::XOF,
                None
            ),
            Country::BES => (
                "Bonaire, Sint Eustatius and Saba",
                "Caribbean Netherlands",
                "BQ",
                "BES",
                535,
                NationalCurrency::USD,
                None
            ),
            Country::BFA => (
                "Burkina Faso",
                "Burkina Faso",
                "BF",
                "BFA",
                854,
                NationalCurrency::XOF,
                None
            ),
            Country::BGD => (
                "People's Republic of Bangladesh",
                "Bangladesh",
                "BD",
                "BGD",
                050,
                NationalCurrency::BDT,
                Some(Decimal::from_str("15.0").unwrap())
            ),
            Country::BGR => (
                "Republic of Bulgaria",
                "Bulgaria",
                "BG",
                "BGR",
                100,
                NationalCurrency::BGN,
                None
            ),
            Country::BHR => (
                "Kingdom of Bahrain",
                "Bahrain",
                "BH",
                "BHR",
                048,
                NationalCurrency::BHD,
                None
            ),
            Country::BHS => (
                "Commonwealth of The Bahamas",
                "Bahamas",
                "BS",
                "BHS",
                044,
                NationalCurrency::BSD,
                Some(Decimal::from_str("12.0").unwrap())
            ),
            Country::BIH => (
                "Bosnia and Herzegovina",
                "Bosnia and Herzegovina",
                "BA",
                "BIH",
                070,
                NationalCurrency::BAM,
                None
            ),
            Country::BLM => (
                "Saint Barthélemy",
                "Saint Barthélemy",
                "BL",
                "BLM",
                652,
                NationalCurrency::EUR,
                None
            ),
            Country::BLR => (
                "Republic of Belarus",
                "Belarus",
                "BY",
                "BLR",
                112,
                NationalCurrency::BYR,
                Some(Decimal::from_str("20.0").unwrap())
            ),
            Country::BLZ => (
                "Belize",
                "Belize",
                "BZ",
                "BLZ",
                084,
                NationalCurrency::BZD,
                None
            ),
            Country::BMU => (
                "Bermuda",
                "Bermuda",
                "BM",
                "BMU",
                060,
                NationalCurrency::BMD,
                None
            ),
            Country::BOL => (
                "Plurinational State of Bolivia",
                "Bolivia",
                "BO",
                "BOL",
                068,
                NationalCurrency::BOB,
                None
            ),
            Country::BRA => (
                "Federative Republic of Brazil",
                "Brazil",
                "BR",
                "BRA",
                076,
                NationalCurrency::BRL,
                Some(Decimal::from_str("2.0").unwrap())
            ),
            Country::BRB => (
                "Barbados",
                "Barbados",
                "BB",
                "BRB",
                052,
                NationalCurrency::BBD,
                None
            ),
            Country::BRN => (
                "Nation of Brunei, the Abode of Peace",
                "Brunei",
                "BN",
                "BRN",
                096,
                NationalCurrency::BND,
                None
            ),
            Country::BTN => (
                "Kingdom of Bhutan",
                "Bhutan",
                "BT",
                "BTN",
                064,
                NationalCurrency::BTN,
                None
            ),
            Country::BVT => (
                "Bouvet Island",
                "Bouvet Island",
                "BV",
                "BVT",
                074,
                NationalCurrency::NOK,
                None
            ),
            Country::BWA => (
                "Republic of Botswana",
                "Botswana",
                "BW",
                "BWA",
                072,
                NationalCurrency::BWP,
                None
            ),
            Country::CAF => (
                "Central African Republic",
                "Central African Republic",
                "CF",
                "CAF",
                140,
                NationalCurrency::XAF,
                None
            ),
            Country::CAN => (
                "Canada",
                "Canada",
                "CA",
                "CAN",
                124,
                NationalCurrency::CAD,
                Some(Decimal::from_str("5.0").unwrap())
            ),
            Country::CCK => (
                "Territory of Cocos {Keeling} Islands",
                "Cocos {Keeling} Islands",
                "CC",
                "CCK",
                166,
                NationalCurrency::AUD,
                Some(Decimal::from_str("10.0").unwrap())
            ),
            Country::CHE => (
                "Swiss Confederation",
                "Switzerland",
                "CH",
                "CHE",
                756,
                NationalCurrency::CHF,
                None
            ),
            Country::CHL => (
                "Republic of Chile",
                "Chile",
                "CL",
                "CHL",
                152,
                NationalCurrency::CLP,
                Some(Decimal::from_str("19.0").unwrap())
            ),
            Country::CHN => (
                "People's Republic of China",
                "China",
                "CN",
                "CHN",
                156,
                NationalCurrency::CNY,
                Some(Decimal::from_str("6.0").unwrap())
            ),
            Country::CIV => (
                "Republic of Côte d'Ivoire",
                "Côte d'Ivoire",
                "CI",
                "CIV",
                384,
                NationalCurrency::XOF,
                None
            ),
            Country::CMR => (
                "Republic of Cameroon",
                "Cameroon",
                "CM",
                "CMR",
                120,
                NationalCurrency::XAF,
                Some(Decimal::from_str("20.0").unwrap())
            ),
            Country::COD => (
                "Democratic Republic of the Congo",
                "Congo",
                "CD",
                "COD",
                180,
                NationalCurrency::XAF,
                None
            ),
            Country::COG => (
                "Republic of the Congo",
                "Congo Republic",
                "CG",
                "COG",
                178,
                NationalCurrency::XAF,
                None
            ),
            Country::COK => (
                "Cook Islands",
                "Cook Islands",
                "CK",
                "COK",
                184,
                NationalCurrency::NZD,
                None
            ),
            Country::COL => (
                "Republic of Colombia",
                "Colombia",
                "CO",
                "COL",
                170,
                NationalCurrency::COP,
                Some(Decimal::from_str("19.0").unwrap())
            ),
            Country::COM => (
                "Union of the Comoros",
                "Comoros",
                "KM",
                "COM",
                174,
                NationalCurrency::KMF,
                None
            ),
            Country::CPV => (
                "Republic of Cabo Verde",
                "Cape Verde",
                "CV",
                "CPV",
                132,
                NationalCurrency::CVE,
                None
            ),
            Country::CPI => (
                "Republic of Costa Rica",
                "Costa Rica",
                "CR",
                "CPI",
                188,
                NationalCurrency::CRC,
                Some(Decimal::from_str("19.0").unwrap())
            ),
            Country::CUB => (
                "Republic of Cuba",
                "Cuba",
                "CU",
                "CUB",
                192,
                NationalCurrency::CUP,
                Some(Decimal::from_str("12.0").unwrap())
            ),
            Country::CUW => (
                "Curaçao",
                "Curaçao",
                "CW",
                "CUW",
                531,
                NationalCurrency::ANG,
                None
            ),
            Country::CXR => (
                "Territory of Christmas Island",
                "Christmas Island",
                "CX",
                "CXR",
                162,
                NationalCurrency::AUD,
                Some(Decimal::from_str("10.0").unwrap())
            ),
            Country::CYM => (
                "Cayman Islands",
                "Cayman Islands",
                "CX",
                "CYM",
                136,
                NationalCurrency::KYD,
                None
            ),
            Country::CYP => (
                "Republic of Cyprus",
                "Cyprus",
                "CY",
                "CYP",
                196,
                NationalCurrency::EUR,
                Some(Decimal::from_str("19.0").unwrap())
            ),
            Country::CZE => (
                "Czech Republic",
                "Czechia",
                "CZ",
                "CZE",
                203,
                NationalCurrency::CZK,
                None
            ),
            Country::DEU => (
                "Federal Republic of Germany",
                "Germany",
                "DE",
                "DEU",
                276,
                NationalCurrency::EUR,
                Some(Decimal::from_str("19.0").unwrap())
            ),
            Country::DJI => (
                "Republic of Djibouti",
                "Djibouti",
                "DJ",
                "DJI",
                262,
                NationalCurrency::DJF,
                None
            ),
            Country::DMA => (
                "Commonwealth of Dominica",
                "Dominica",
                "DM",
                "DMA",
                212,
                NationalCurrency::DOP,
                None
            ),
            Country::DNK => (
                "Kingdom of Denmark",
                "Denmark",
                "DK",
                "DNK",
                208,
                NationalCurrency::DKK,
                None
            ),
            Country::DOM => (
                "Dominican Republic",
                "Dominican Republic",
                "DO",
                "DOM",
                214,
                NationalCurrency::DOP,
                None
            ),
            Country::DZA => (
                "People's Democratic Republic of Algeria",
                "Algeria",
                "DZ",
                "DZA",
                012,
                NationalCurrency::DZD,
                Some(Decimal::from_str("9.0").unwrap())
            ),
            Country::ECU => (
                "Republic of Ecuador",
                "Ecuador",
                "EC",
                "ECU",
                218,
                NationalCurrency::USD,
                None
            ),
            Country::EGY => (
                "Arab Republic of Egypt",
                "Egypt",
                "EG",
                "EGY",
                818,
                NationalCurrency::EGP,
                Some(Decimal::from_str("14.0").unwrap())
            ),
            Country::ERI => (
                "State of Eritrea",
                "Eritrea",
                "ER",
                "ERI",
                232,
                NationalCurrency::ERN,
                None
            ),
            Country::ESP => (
                "Kingdom of Spain",
                "Spain",
                "ES",
                "ESP",
                724,
                NationalCurrency::EUR,
                Some(Decimal::from_str("21.0").unwrap())
            ),
            Country::EST => (
                "Republic of Estonia",
                "Estonia",
                "EE",
                "EST",
                233,
                NationalCurrency::EUR,
                Some(Decimal::from_str("20.0").unwrap())
            ),
            Country::ETH => (
                "Federal Democratic Republic of Ethiopia",
                "Ethiopia",
                "ET",
                "ETH",
                231,
                NationalCurrency::ETB,
                None
            ),
            Country::FIN => (
                "Republic of Finland",
                "Finland",
                "FI",
                "FIN",
                246,
                NationalCurrency::EUR,
                Some(Decimal::from_str("24.0").unwrap())
            ),
            Country::FJI => (
                "Republic of Fiji",
                "Fiji",
                "FJ",
                "FJI",
                242,
                NationalCurrency::FJD,
                None
            ),
            Country::FLK => (
                "Falkland Islands",
                "Falkland Islands",
                "FK",
                "FLK",
                238,
                NationalCurrency::FKP,
                None
            ),
            Country::FRA => (
                "French Republic",
                "France",
                "FR",
                "FRA",
                250,
                NationalCurrency::EUR,
                Some(Decimal::from_str("20.0").unwrap())
            ),
            Country::FRO => (
                "Faroe Islands",
                "Faroe Islands",
                "FO",
                "FRO",
                234,
                NationalCurrency::DKK,
                None
            ),
            Country::FSM => (
                "Federated States of Micronesia",
                "Micronesia",
                "FM",
                "FSM",
                583,
                NationalCurrency::USD,
                None
            ),
            Country::GAB => (
                "Gabonese Republic",
                "Gabon",
                "GA",
                "GAB",
                266,
                NationalCurrency::XAF,
                None
            ),
            Country::GBR => (
                "United Kingdom of Great Britain and Northern Ireland",
                "United Kingdom",
                "GB",
                "GBR",
                826,
                NationalCurrency::GBP,
                None
            ),
            Country::GEO => (
                "Georgia",
                "Georgia",
                "GE",
                "GEO",
                268,
                NationalCurrency::GEL,
                None
            ),
            Country::GGY => (
                "Bailiwick of Guernsey",
                "Guernsey",
                "GG",
                "GGY",
                831,
                NationalCurrency::GBP,
                None
            ),
            Country::GHA => (
                "Republic of Ghana",
                "Ghana",
                "GH",
                "GHA",
                288,
                NationalCurrency::GHS,
                Some(Decimal::from_str("17.0").unwrap())
            ),
            Country::GIB => (
                "Gibraltar",
                "Gibraltar",
                "GI",
                "GIB",
                292,
                NationalCurrency::GIP,
                None
            ),
            Country::GIN => (
                "Republic of Guinea",
                "Guinea",
                "GN",
                "GIN",
                324,
                NationalCurrency::GNF,
                None
            ),
            Country::GLP => (
                "Guadeloupe",
                "Guadeloupe",
                "GP",
                "GLP",
                312,
                NationalCurrency::EUR,
                None
            ),
            Country::GMB => (
                "Republic of the Gambia",
                "Gambia",
                "GM",
                "GMB",
                270,
                NationalCurrency::GMD,
                None
            ),
            Country::GNB => (
                "Republic of Guinea-Bissau",
                "Guinea-Bissau",
                "GW",
                "GNB",
                624,
                NationalCurrency::XOF,
                None
            ),
            Country::GNQ => (
                "Republic of Equatorial Guinea",
                "Equatorial Guinea",
                "GQ",
                "GNQ",
                226,
                NationalCurrency::PGK,
                None
            ),
            Country::GRC => (
                "Hellenic Republic",
                "Greece",
                "GR",
                "GRC",
                300,
                NationalCurrency::EUR,
                Some(Decimal::from_str("24.0").unwrap())
            ),
            Country::GRD => (
                "Grenada",
                "Grenada",
                "GD",
                "GRD",
                308,
                NationalCurrency::XCD,
                None
            ),
            Country::GRL => (
                "Greenland",
                "Greenland",
                "GL",
                "GRL",
                304,
                NationalCurrency::DKK,
                None
            ),
            Country::GTM => (
                "Republic of Guatemala",
                "Guatemala",
                "GT",
                "GTM",
                320,
                NationalCurrency::GTQ,
                None
            ),
            Country::GUF => (
                "French Guiana",
                "French Guiana",
                "GF",
                "GUF",
                254,
                NationalCurrency::EUR,
                None
            ),
            Country::GUM => (
                "Guam",
                "Guam",
                "GU",
                "GUM",
                316,
                NationalCurrency::USD,
                None
            ),
            Country::GUY => (
                "Co‑operative Republic of Guyana",
                "Guyana",
                "GY",
                "GUY",
                328,
                NationalCurrency::GYD,
                None
            ),
            Country::HKG => (
                "Hong Kong Special Administrative Region of the People's Republic of China",
                "Hong Kong",
                "HK",
                "HKG",
                344,
                NationalCurrency::HKD,
                None
            ),
            Country::HMD => (
                "Territory of Heard Island and McDonald Islands",
                "Heard Island and McDonald Islands",
                "HM",
                "HMD",
                334,
                NationalCurrency::AUD,
                Some(Decimal::from_str("10.0").unwrap())
            ),
            Country::HND => (
                "Republic of Honduras",
                "Honduras",
                "HN",
                "HND",
                340,
                NationalCurrency::HNL,
                None
            ),
            Country::HRV => (
                "Republic of Croatia",
                "Croatia",
                "HR",
                "HRV",
                191,
                NationalCurrency::HRK,
                None
            ),
            Country::HTI => (
                "Republic of Haiti",
                "Haiti",
                "HT",
                "HTI",
                332,
                NationalCurrency::HTG,
                None
            ),
            Country::HUN => (
                "Hungary",
                "Hungary",
                "HU",
                "HUN",
                348,
                NationalCurrency::HUF,
                None
            ),
            Country::IDN => (
                "Republic of Indonesia",
                "Indonesia",
                "ID",
                "IDN",
                360,
                NationalCurrency::IDR,
                Some(Decimal::from_str("10.0").unwrap())
            ),
            Country::IMN => (
                "Isle of Man",
                "Isle of Man",
                "IM",
                "IMN",
                833,
                NationalCurrency::GBP,
                None
            ),
            Country::IND => (
                "Republic of India",
                "India",
                "IN",
                "IND",
                356,
                NationalCurrency::INR,
                Some(Decimal::from_str("18.0").unwrap())
            ),
            Country::IOT => (
                "British Indian Ocean Territory",
                "British Indian Ocean Territory",
                "IO",
                "IOT",
                086,
                NationalCurrency::GBP,
                None
            ),
            Country::IRL => (
                "Republic of Ireland",
                "Ireland",
                "IE",
                "IRL",
                372,
                NationalCurrency::EUR,
                Some(Decimal::from_str("9.0").unwrap())
            ),
            Country::IRN => (
                "Islamic Republic of Iran",
                "Iran",
                "IR",
                "IRN",
                364,
                NationalCurrency::IRR,
                None
            ),
            Country::IRQ => (
                "Republic of Iraq",
                "Iraq",
                "IQ",
                "IRQ",
                368,
                NationalCurrency::IQD,
                None
            ),
            Country::ISL => (
                "Iceland",
                "Iceland",
                "IS",
                "ISL",
                352,
                NationalCurrency::ISK,
                Some(Decimal::from_str("24.0").unwrap())
            ),
            Country::ISR => (
                "State of Israel",
                "Israel",
                "IL",
                "ISR",
                376,
                NationalCurrency::ILS,
                None
            ),
            Country::ITA => (
                "Italian Republic",
                "Italy",
                "IT",
                "ITA",
                380,
                NationalCurrency::EUR,
                Some(Decimal::from_str("22.0").unwrap())
            ),
            Country::JAM => (
                "Jamaica",
                "Jamaica",
                "JM",
                "JAM",
                388,
                NationalCurrency::JMD,
                None
            ),
            Country::JEY => (
                "Bailiwick of Jersey",
                "Jersey",
                "JE",
                "JEY",
                832,
                NationalCurrency::GBP,
                None
            ),
            Country::JOR => (
                "Hashemite Kingdom of Jordan",
                "Jordan",
                "JO",
                "JOR",
                400,
                NationalCurrency::JOD,
                None
            ),
            Country::JPN => (
                "Japan",
                "Japan",
                "JP",
                "JPN",
                392,
                NationalCurrency::JPY,
                Some(Decimal::from_str("10.0").unwrap())
            ),
            Country::KAZ => (
                "Republic of Kazakhstan",
                "Kazakhstan",
                "KZ",
                "KAZ",
                398,
                NationalCurrency::KZT,
                Some(Decimal::from_str("12.0").unwrap())
            ),
            Country::KEN => (
                "Republic of Kenya",
                "Kenya",
                "KE",
                "KEN",
                404,
                NationalCurrency::KES,
                Some(Decimal::from_str("14.0").unwrap())
            ),
            Country::KGZ => (
                "Kyrgyz Republic",
                "Kyrgyzstan",
                "KG",
                "KGZ",
                417,
                NationalCurrency::KGS,
                None
            ),
            Country::KHM => (
                "Kingdom of Cambodia",
                "Cambodia",
                "KH",
                "KHM",
                116,
                NationalCurrency::KHR,
                None
            ),
            Country::KIR => (
                "Republic of Kiribati",
                "Kiribati",
                "KI",
                "KIR",
                296,
                NationalCurrency::AUD,
                Some(Decimal::from_str("10.0").unwrap())
            ),
            Country::KNA => (
                "Federation of Saint Christopher and Nevis",
                "Saint Kitts and Nevis",
                "KN",
                "KNA",
                659,
                NationalCurrency::XCD,
                None
            ),
            Country::KOR => (
                "Republic of Korea",
                "South Korea",
                "KR",
                "KOR",
                410,
                NationalCurrency::KRW,
                None
            ),
            Country::KWT => (
                "State of Kuwait",
                "Kuwait",
                "KW",
                "KWT",
                414,
                NationalCurrency::KWD,
                None
            ),
            Country::LAO => (
                "Lao People's Democratic Republic",
                "Laos",
                "LA",
                "LAO",
                418,
                NationalCurrency::LAK,
                None
            ),
            Country::LBN => (
                "Lebanese Republic",
                "Lebanon",
                "LB",
                "LBN",
                422,
                NationalCurrency::LBP,
                None
            ),
            Country::LBR => (
                "Republic of Liberia",
                "Liberia",
                "LR",
                "LBR",
                430,
                NationalCurrency::LRD,
                None
            ),
            Country::LBY => (
                "State of Libya",
                "Libya",
                "LY",
                "LBY",
                434,
                NationalCurrency::LYD,
                None
            ),
            Country::LCA => (
                "Saint Lucia",
                "Saint Lucia",
                "LC",
                "LCA",
                662,
                NationalCurrency::XCD,
                None
            ),
            Country::LIE => (
                "Principality of Liechtenstein",
                "Liechtenstein",
                "LI",
                "LIE",
                438,
                NationalCurrency::CHF,
                None
            ),
            Country::LKA => (
                "Democratic Socialist Republic of Sri Lanka",
                "Sri Lanka",
                "LK",
                "LKA",
                144,
                NationalCurrency::LKR,
                None
            ),
            Country::LSO => (
                "Kingdom of Lesotho",
                "Lesotho",
                "LS",
                "LSO",
                426,
                NationalCurrency::ZAR,
                None
            ),
            Country::LTU => (
                "Republic of Lithuania",
                "Lithuania",
                "LT",
                "LTU",
                440,
                NationalCurrency::EUR,
                Some(Decimal::from_str("21.0").unwrap())
            ),
            Country::LUX => (
                "Grand Duchy of Luxembourg",
                "Luxembourg",
                "LU",
                "LUX",
                442,
                NationalCurrency::EUR,
                Some(Decimal::from_str("17.0").unwrap())
            ),
            Country::LVA => (
                "Republic of Latvia",
                "Latvia",
                "LV",
                "LVA",
                428,
                NationalCurrency::EUR,
                Some(Decimal::from_str("21.0").unwrap())
            ),
            Country::MAC => (
                "Macao Special Administrative Region of the People's Republic of China",
                "Macau",
                "MO",
                "MAC",
                446,
                NationalCurrency::MOP,
                None
            ),
            Country::MAF => (
                "Collectivity of Saint Martin",
                "Saint Martin",
                "MF",
                "MAF",
                663,
                NationalCurrency::EUR,
                None
            ),
            Country::MAR => (
                "Kingdom of Morocco",
                "Morocco",
                "MA",
                "MAR",
                504,
                NationalCurrency::MAD,
                Some(Decimal::from_str("20.0").unwrap())
            ),
            Country::MCO => (
                "Principality of Monaco",
                "Monaco",
                "MC",
                "MCO",
                492,
                NationalCurrency::EUR,
                Some(Decimal::from_str("19.5").unwrap())
            ),
            Country::MDA => (
                "Republic of Moldova",
                "Moldova",
                "MD",
                "MDA",
                498,
                NationalCurrency::MDL,
                Some(Decimal::from_str("20.0").unwrap())
            ),
            Country::MDG => (
                "Republic of Madagascar",
                "Madagascar",
                "MG",
                "MDG",
                450,
                NationalCurrency::MGA,
                None
            ),
            Country::MDV => (
                "Republic of Maldives",
                "Maldives",
                "MV",
                "MDV",
                462,
                NationalCurrency::MVR,
                None
            ),
            Country::MEX => (
                "United Mexican States",
                "Mexico",
                "MX",
                "MEX",
                484,
                NationalCurrency::MXN,
                Some(Decimal::from_str("16.0").unwrap())
            ),
            Country::MHL => (
                "Republic of the Marshall Islands",
                "Marshall Islands",
                "MH",
                "MHL",
                584,
                NationalCurrency::USD,
                None
            ),
            Country::MKD => (
                "Republic of North Macedonia",
                "North Macedonia",
                "MK",
                "MKD",
                807,
                NationalCurrency::MKD,
                None
            ),
            Country::MLI => (
                "Republic of Mali",
                "Mali",
                "ML",
                "MLI",
                466,
                NationalCurrency::XOF,
                None
            ),
            Country::MLT => (
                "Republic of Malta",
                "Malta",
                "MT",
                "MLT",
                470,
                NationalCurrency::EUR,
                None
            ),
            Country::MMR => (
                "Republic of the Union of Myanmar",
                "Myanmar",
                "MM",
                "MMR",
                104,
                NationalCurrency::MMK,
                None
            ),
            Country::MNE => (
                "Montenegro",
                "Montenegro",
                "ME",
                "MNE",
                499,
                NationalCurrency::EUR,
                Some(Decimal::from_str("21.0").unwrap())
            ),
            Country::MNG => (
                "Mongolia",
                "Mongolia",
                "MN",
                "MNG",
                496,
                NationalCurrency::MNT,
                None
            ),
            Country::MNP => (
                "Commonwealth of the Northern Mariana Islands",
                "Northern Mariana Islands",
                "MP",
                "MNP",
                580,
                NationalCurrency::USD,
                None
            ),
            Country::MOZ => (
                "Republic of Mozambique",
                "Mozambique",
                "MZ",
                "MOZ",
                508,
                NationalCurrency::MZN,
                None
            ),
            Country::MRT => (
                "Islamic Republic of Mauritania",
                "Mauritania",
                "MR",
                "MRT",
                478,
                NationalCurrency::MRO,
                None
            ),
            Country::MSR => (
                "Montserrat",
                "Montserrat",
                "MS",
                "MSR",
                500,
                NationalCurrency::XCD,
                None
            ),
            Country::MTQ => (
                "Martinique",
                "Martinique",
                "MQ",
                "MTQ",
                474,
                NationalCurrency::EUR,
                None
            ),
            Country::MUS => (
                "Republic of Mauritius",
                "Mauritius",
                "MU",
                "MUS",
                480,
                NationalCurrency::MUR,
                Some(Decimal::from_str("15.0").unwrap())
            ),
            Country::MWI => (
                "Republic of Malawi",
                "Malawi",
                "MW",
                "MWI",
                454,
                NationalCurrency::MWK,
                None
            ),
            Country::MYS => (
                "Malaysia",
                "Malaysia",
                "MY",
                "MYS",
                458,
                NationalCurrency::MYR,
                Some(Decimal::from_str("6.0").unwrap())
            ),
            Country::MYT => (
                "Department of Mayotte",
                "Mayotte",
                "YT",
                "MYT",
                175,
                NationalCurrency::EUR,
                None
            ),
            Country::NAM => (
                "Republic of Namibia",
                "Namibia",
                "NA",
                "NAM",
                516,
                NationalCurrency::NAD,
                None
            ),
            Country::NCL => (
                "New Caledonia",
                "New Caledonia",
                "NC",
                "NCL",
                540,
                NationalCurrency::XPF,
                None
            ),
            Country::NER => (
                "Republic of the Niger",
                "Niger",
                "NE",
                "NER",
                562,
                NationalCurrency::XOF,
                None
            ),
            Country::NFK => (
                "Norfolk Island",
                "Norfolk Island",
                "NF",
                "NFK",
                574,
                NationalCurrency::AUD,
                Some(Decimal::from_str("10.0").unwrap())
            ),
            Country::NGA => (
                "Federal Republic of Nigeria",
                "Nigeria",
                "NG",
                "NGA",
                566,
                NationalCurrency::NGN,
                None
            ),
            Country::NIC => (
                "Republic of Nicaragua",
                "Nicaragua",
                "NI",
                "NIC",
                558,
                NationalCurrency::NIO,
                None
            ),
            Country::NIU => (
                "Niue",
                "Niue",
                "NU",
                "NIU",
                570,
                NationalCurrency::NZD,
                None
            ),
            Country::NLD => (
                "Netherlands",
                "Netherlands",
                "NL",
                "NLD",
                528,
                NationalCurrency::EUR,
                Some(Decimal::from_str("21.0").unwrap())
            ),
            Country::NOR => (
                "Kingdom of Norway",
                "Norway",
                "NO",
                "NOR",
                578,
                NationalCurrency::NOK,
                None
            ),
            Country::NPL => (
                "Federal Democratic Republic of Nepal",
                "Nepal",
                "NP",
                "NPL",
                524,
                NationalCurrency::NPR,
                None
            ),
            Country::NRU => (
                "Republic of Nauru",
                "Nauru",
                "NR",
                "NRU",
                520,
                NationalCurrency::AUD,
                Some(Decimal::from_str("10.0").unwrap())
            ),
            Country::NZL => (
                "New Zealand",
                "New Zealand",
                "NZ",
                "NZL",
                554,
                NationalCurrency::NZD,
                None
            ),
            Country::OMN => (
                "Sultanate of Oman",
                "Oman",
                "OM",
                "OMN",
                512,
                NationalCurrency::OMR,
                None
            ),
            Country::PAK => (
                "Islamic Republic of Pakistan",
                "Pakistan",
                "PK",
                "PAK",
                586,
                NationalCurrency::PKR,
                None
            ),
            Country::PAN => (
                "Republic of Panama",
                "Panama",
                "PA",
                "PAN",
                591,
                NationalCurrency::PAB,
                None
            ),
            Country::PCN => (
                "Pitcairn, Henderson, Ducie and Oeno Islands",
                "Pitcairn Islands",
                "PN",
                "PCN",
                612,
                NationalCurrency::NZD,
                None
            ),
            Country::PER => (
                "Republic of Peru",
                "Peru",
                "PE",
                "PER",
                604,
                NationalCurrency::PEN,
                None
            ),
            Country::PHL => (
                "Republic of the Philippines",
                "Philippines",
                "PH",
                "PHL",
                608,
                NationalCurrency::PHP,
                None
            ),
            Country::PLW => (
                "Republic of Palau",
                "Palau",
                "PW",
                "PLW",
                585,
                NationalCurrency::USD,
                None
            ),
            Country::PNG => (
                "Independent State of Papua New Guinea",
                "Papua New Guinea",
                "PG",
                "PNG",
                598,
                NationalCurrency::PGK,
                None
            ),
            Country::POL => (
                "Republic of Poland",
                "Poland",
                "PL",
                "POL",
                616,
                NationalCurrency::PLN,
                None
            ),
            Country::PRI => (
                "Commonwealth of Puerto Rico",
                "Puerto Rico",
                "PR",
                "PRI",
                630,
                NationalCurrency::USD,
                None
            ),
            Country::PRK => (
                "Democratic People's Republic of Korea",
                "North Korea",
                "KP",
                "PRK",
                408,
                NationalCurrency::KPW,
                None
            ),
            Country::PRT => (
                "Portuguese Republic",
                "Portugal",
                "PT",
                "PRT",
                620,
                NationalCurrency::EUR,
                Some(Decimal::from_str("18.0").unwrap())
            ),
            Country::PRY => (
                "Republic of Paraguay",
                "Paraguay",
                "PY",
                "PRY",
                600,
                NationalCurrency::PYG,
                None
            ),
            Country::PSE => (
                "State of Palestine",
                "Palestine",
                "PS",
                "PSE",
                275,
                NationalCurrency::JOD,
                None
            ),
            Country::PYF => (
                "French Polynesia",
                "French Polynesia",
                "PF",
                "PYF",
                258,
                NationalCurrency::XPF,
                None
            ),
            Country::QAT => (
                "State of Qatar",
                "Qatar",
                "QA",
                "QAT",
                634,
                NationalCurrency::QAR,
                None
            ),
            Country::REU => (
                "Réunion",
                "Réunion",
                "RE",
                "REU",
                638,
                NationalCurrency::EUR,
                None
            ),
            Country::ROU => (
                "Romania",
                "Romania",
                "RO",
                "ROU",
                642,
                NationalCurrency::RON,
                None
            ),
            Country::RUS => (
                "Russian Federation",
                "Russia",
                "RU",
                "RUS",
                643,
                NationalCurrency::RUB,
                None
            ),
            Country::RWA => (
                "Republic of Rwanda",
                "Rwanda",
                "RW",
                "RWA",
                646,
                NationalCurrency::RWF,
                None
            ),
            Country::SAU => (
                "Kingdom of Saudi Arabia",
                "Saudi Arabia",
                "SA",
                "SAU",
                682,
                NationalCurrency::SAR,
                None
            ),
            Country::SDN => (
                "Republic of the Sudan",
                "Sudan",
                "SD",
                "SDN",
                729,
                NationalCurrency::SDG,
                None
            ),
            Country::SEN => (
                "Republic of Senegal",
                "Senegal",
                "SN",
                "SEN",
                686,
                NationalCurrency::XOF,
                None
            ),
            Country::SGP => (
                "Republic of Singapore",
                "Singapore",
                "SG",
                "SGP",
                702,
                NationalCurrency::SGD,
                None
            ),
            Country::SGS => (
                "South Georgia and the South Sandwich Islands",
                "South Georgia and the South Sandwich Islands",
                "GS",
                "SGS",
                239,
                NationalCurrency::GEL,
                None
            ),
            Country::SHN => (
                "Saint Helena, Ascension and Tristan da Cunha",
                "Saint Helena, Ascension and Tristan da Cunha",
                "SH",
                "SHN",
                654,
                NationalCurrency::SHP,
                None
            ),
            Country::SJM => (
                "Svalbard and Jan Mayen",
                "Svalbard and Jan Mayen",
                "SJ",
                "SJM",
                744,
                NationalCurrency::NOK,
                None
            ),
            Country::SLB => (
                "Solomon Islands",
                "Solomon Islands",
                "SB",
                "SLB",
                090,
                NationalCurrency::SBD,
                None
            ),
            Country::SLE => (
                "Republic of Sierra Leone",
                "Sierra Leone",
                "SL",
                "SLE",
                694,
                NationalCurrency::SLL,
                None
            ),
            Country::SLV => (
                "Republic of El Salvador",
                "El Salvador",
                "SV",
                "SLV",
                222,
                NationalCurrency::USD,
                None
            ),
            Country::SMR => (
                "Republic of San Marino",
                "San Marino",
                "SM",
                "SMR",
                674,
                NationalCurrency::EUR,
                None
            ),
            Country::SOM => (
                "Federal Republic of Somalia",
                "Somalia",
                "SO",
                "SOM",
                706,
                NationalCurrency::SOS,
                None
            ),
            Country::SPM => (
                "Territorial Collectivity of Saint-Pierre and Miquelon",
                "Saint Pierre and Miquelon",
                "PM",
                "SPM",
                666,
                NationalCurrency::EUR,
                None
            ),
            Country::SRB => (
                "Republic of Serbia",
                "Serbia",
                "RS",
                "SRB",
                688,
                NationalCurrency::RSD,
                None
            ),
            Country::SSD => (
                "Republic of South Sudan",
                "South Sudan",
                "SS",
                "SSD",
                728,
                NationalCurrency::SSP,
                None
            ),
            Country::STP => (
                "Democratic Republic of São Tomé and Príncipe",
                "Sao Tome and Principe",
                "ST",
                "STP",
                678,
                NationalCurrency::STD,
                None
            ),
            Country::SUR => (
                "Republic of Suriname",
                "Suriname",
                "SR",
                "SUR",
                740,
                NationalCurrency::SRD,
                None
            ),
            Country::SVK => (
                "Slovak Republic",
                "Slovakia",
                "SK",
                "SVK",
                703,
                NationalCurrency::EUR,
                Some(Decimal::from_str("20.0").unwrap())
            ),
            Country::SVN => (
                "Republic of Slovenia",
                "Slovenia",
                "SI",
                "SVN",
                705,
                NationalCurrency::EUR,
                Some(Decimal::from_str("22.0").unwrap())
            ),
            Country::SWE => (
                "Kingdom of Sweden",
                "Sweden",
                "SE",
                "SWE",
                752,
                NationalCurrency::SEK,
                None
            ),
            Country::SWZ => (
                "Kingdom of Eswatini",
                "Eswatini",
                "SZ",
                "SWZ",
                748,
                NationalCurrency::SZL,
                None
            ),
            Country::SXM => (
                "Sint Maarten",
                "Sint Maarten",
                "SX",
                "SXM",
                534,
                NationalCurrency::ANG,
                None
            ),
            Country::SYC => (
                "Republic of Seychelles",
                "Seychelles",
                "SC",
                "SYC",
                690,
                NationalCurrency::SCR,
                None
            ),
            Country::SYR => (
                "Syrian Arab Republic",
                "Syria",
                "SY",
                "SYR",
                760,
                NationalCurrency::SYP,
                None
            ),
            Country::TCA => (
                "Turks and Caicos Islands",
                "Turks and Caicos Islands",
                "TC",
                "TCA",
                796,
                NationalCurrency::USD,
                None
            ),
            Country::TCD => (
                "Republic of Chad",
                "Chad",
                "TD",
                "TCD",
                148,
                NationalCurrency::XAF,
                None
            ),
            Country::TGO => (
                "Togolese Republic",
                "Togo",
                "TG",
                "TGO",
                768,
                NationalCurrency::XOF,
                None
            ),
            Country::THA => (
                "Kingdom of Thailand",
                "Thailand",
                "TH",
                "THA",
                764,
                NationalCurrency::THB,
                None
            ),
            Country::TJK => (
                "Republic of Tajikistan",
                "Tajikistan",
                "TJ",
                "TJK",
                762,
                NationalCurrency::TJS,
                None
            ),
            Country::TKL => (
                "Tokelau Islands",
                "Tokelau",
                "TK",
                "TKL",
                772,
                NationalCurrency::NZD,
                None
            ),
            Country::TKM => (
                "Turkmenistan",
                "Turkmenistan",
                "TM",
                "TKM",
                795,
                NationalCurrency::TMT,
                None
            ),
            Country::TLS => (
                "Democratic Republic of Timor-Leste",
                "East Timor",
                "TL",
                "TLS",
                626,
                NationalCurrency::USD,
                None
            ),
            Country::TON => (
                "Kingdom of Tonga",
                "Tonga",
                "TO",
                "TON",
                776,
                NationalCurrency::TOP,
                None
            ),
            Country::TTO => (
                "Republic of Trinidad and Tobago",
                "Trinidad and Tobago",
                "TT",
                "TTO",
                780,
                NationalCurrency::TTD,
                None
            ),
            Country::TUN => (
                "Republic of Tunisia",
                "Tunisia",
                "TN",
                "TUN",
                788,
                NationalCurrency::TND,
                None
            ),
            Country::TUR => (
                "Republic of Turkey",
                "Turkey",
                "TR",
                "TUR",
                792,
                NationalCurrency::TRY,
                None
            ),
            Country::TUV => (
                "Tuvalu",
                "Tuvalu",
                "TV",
                "TUV",
                798,
                NationalCurrency::AUD,
                Some(Decimal::from_str("10.0").unwrap())
            ),
            Country::TWN => (
                "Republic of China",
                "Taiwan",
                "TW",
                "TWN",
                158,
                NationalCurrency::TWD,
                None
            ),
            Country::TZA => (
                "United Republic of Tanzania",
                "Tanzania",
                "TZ",
                "TZA",
                834,
                NationalCurrency::TZS,
                None
            ),
            Country::UGA => (
                "Republic of Ugand",
                "Uganda",
                "UG",
                "UGA",
                800,
                NationalCurrency::UGX,
                None
            ),
            Country::UKR => (
                "Ukraine",
                "Ukraine",
                "UA",
                "UKR",
                804,
                NationalCurrency::UAH,
                None
            ),
            Country::UMI => (
                "United States Minor Outlying Islands",
                "United States Minor Outlying Islands",
                "UM",
                "UMI",
                581,
                NationalCurrency::USD,
                None
            ),
            Country::URY => (
                "Oriental Republic of Uruguay",
                "Uruguay",
                "UY",
                "URY",
                858,
                NationalCurrency::UYU,
                None
            ),
            Country::USA => (
                "United States of America",
                "United States",
                "US",
                "USA",
                840,
                NationalCurrency::USD,
                Some(Decimal::from_str("10.0").unwrap())
            ),
            Country::UZB => (
                "Republic of Uzbekistan",
                "Uzbekistan",
                "UZ",
                "UZB",
                860,
                NationalCurrency::UZS,
                None
            ),
            Country::VAT => (
                "Vatican City State",
                "Vatican City",
                "VA",
                "VAT",
                336,
                NationalCurrency::EUR,
                None
            ),
            Country::VCT => (
                "Saint Vincent and the Grenadines",
                "Saint Vincent",
                "VC",
                "VCT",
                670,
                NationalCurrency::XCD,
                None
            ),
            Country::VEN => (
                "Bolivarian Republic of Venezuela",
                "Venezuela",
                "VE",
                "VEN",
                862,
                NationalCurrency::VEF,
                None
            ),
            Country::VGB => (
                "Virgin Islands",
                "British Virgin Islands",
                "VG",
                "VGB",
                092,
                NationalCurrency::USD,
                None
            ),
            Country::VIR => (
                "Virgin Islands of the United States",
                "United States Virgin Islands",
                "VI",
                "VIR",
                850,
                NationalCurrency::USD,
                None
            ),
            Country::VNM => (
                "Socialist Republic of Vietnam",
                "Vietnam",
                "VN",
                "VNM",
                704,
                NationalCurrency::VND,
                None
            ),
            Country::VUT => (
                "Republic of Vanuatu",
                "Vanuatu",
                "VU",
                "VUT",
                548,
                NationalCurrency::VUV,
                None
            ),
            Country::WLF => (
                "Territory of the Wallis and Futuna Islands",
                "Wallis and Futuna",
                "WF",
                "WLF",
                876,
                NationalCurrency::XPF,
                None
            ),
            Country::WSM => (
                "Independent State of Samoa",
                "Samoa",
                "WS",
                "WSM",
                882,
                NationalCurrency::WST,
                None
            ),
            Country::YEM => (
                "Republic of Yemen",
                "Yemen",
                "YE",
                "YEM",
                887,
                NationalCurrency::YER,
                None
            ),
            Country::ZAF => (
                "Republic of South Africa",
                "South Africa",
                "ZA",
                "ZAF",
                710,
                NationalCurrency::ZAR,
                None
            ),
            Country::ZMB => (
                "Republic of Zambia",
                "Zambia",
                "ZM",
                "ZMB",
                895,
                NationalCurrency::ZMW,
                None
            ),
            Country::ZWE => (
                "Republic of Zimbabwe",
                "Zimbabwe",
                "ZW",
                "ZWE",
                716,
                NationalCurrency::ZWL,
                None
            ),
        }
    }
}

impl FromStr
for Country
{
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for country in Country::iter() {
            if country.to_string().eq_ignore_ascii_case(s) {
                return Ok(country);
            }
            if country.short_name().eq_ignore_ascii_case(s) {
                return Ok(country);
            }
            if country.iso3().eq_ignore_ascii_case(s) {
                return Ok(country);
            }
            if country.iso2().eq_ignore_ascii_case(s) {
                return Ok(country);
            }
        }
        Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Country is not valid: {}.", s)))
    }
}