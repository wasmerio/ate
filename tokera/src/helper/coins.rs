use std::collections::BTreeMap;
use num_traits::*;

use crate::model::*;

pub fn shrink_denomination(mut denomination: Decimal) -> Decimal {
    let scalar = Decimal::new(1000, 0);
    match denomination.checked_mul(scalar).unwrap().to_string() {
        a if a.starts_with("1") => {
            denomination = denomination / Decimal::new(2, 0);
        },
        a if a.starts_with("5") => {
            denomination = denomination * Decimal::new(2, 0);
            denomination = denomination / Decimal::new(5, 0);
        },
        a if a.starts_with("2") => {
            denomination = denomination / Decimal::new(2, 0);
        },
        _ => {
            denomination = denomination / Decimal::new(10, 0);
        }
    }
    denomination
}

pub fn grow_denomination(mut denomination: Decimal) -> Decimal {
    let scalar = Decimal::new(1000, 0);
    match denomination.checked_mul(scalar).unwrap().to_string() {
        a if a.starts_with("1") => {
            denomination *= Decimal::new(2, 0);
        },
        a if a.starts_with("2") => {
            denomination *= Decimal::new(5, 0);
            denomination /= Decimal::new(2, 0);
        },
        a if a.starts_with("5") => {
            denomination *= Decimal::new(2, 0);
        },
        _ => {
            denomination *= Decimal::new(10, 0);
        }
    }
    denomination
}

pub fn carve_denominations(mut amount: Decimal, currency: NationalCurrency) -> BTreeMap<Decimal, usize>
{
    // Select a starting denomination that makes sense
    let lowest_denomination = Decimal::new(1, currency.decimal_points() as u32);
    let mut denomination = lowest_denomination;
    while grow_denomination(denomination) <= amount {
        denomination = grow_denomination(denomination);
    }
    
    // Now lets work the amount down into smaller chunks
    let mut ret = BTreeMap::default();
    while amount > Decimal::zero()
    {
        // Shrink the denomination if its too big
        while denomination > amount && denomination > lowest_denomination {
            denomination = shrink_denomination(denomination);
        }
        if denomination > amount {
            return ret;
        }

        // Record the coin and reduce the amount
        let v = ret.entry(denomination).or_default();
        *v += 1usize;
        amount = amount - denomination;
    }
    
    ret
}