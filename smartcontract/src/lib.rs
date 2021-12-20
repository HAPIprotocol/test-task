use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::Vector;
use near_sdk::{env, log, near_bindgen, setup_alloc};
use std::ops::AddAssign;

const LAST_NUMBERS_FOR_AVERAGE: u64 = 5;

setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct AveragePrice {
    records: Vector<f64>,
}

impl Default for AveragePrice {
    fn default() -> Self {
        Self {
            records: Vector::new::<&[u8]>("qwerty".as_ref()),
        }
    }
}

#[near_bindgen]
impl AveragePrice {
    #[payable]
    pub fn set_last_price(&mut self, price: &f64) {
        if !price.is_normal() {
            env::panic(b"Method set_last_price doesn't accept the number is neither zero, infinite, subnormal, or NaN");
        }
        log!("set_last_price with price {}", price);
        self.records.push(price);
    }

    pub fn get_average_price(&self) -> Option<f64> {
        log!("get_average_price");
        if self.records.len() < LAST_NUMBERS_FOR_AVERAGE {
            let sum: f64 = self.records.iter().sum();
            if sum == 0.0 {
                env::panic(b"No records. Unable to calculate average value.");
            }
            Some(dbg!(sum) / dbg!(self.records.len() as f64))
        } else {
            let mut sum = 0_f64;
            for index in (self.records.len() - LAST_NUMBERS_FOR_AVERAGE)..self.records.len() {
                let value = self
                    .records
                    .get(index)
                    .expect("Unexpected error: Array index out of bounds.");
                sum.add_assign(value);
            }
            Some(sum / LAST_NUMBERS_FOR_AVERAGE as f64)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{get_logs, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};
    use std::convert::TryInto;

    fn get_context(is_view: bool) -> VMContext {
        VMContextBuilder::new()
            .signer_account_id("vkarnaukhov.testnet".try_into().unwrap())
            .is_view(is_view)
            .build()
    }

    #[test]
    #[should_panic]
    fn set_nan_value() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = AveragePrice::default();
        contract.set_last_price(&f64::NAN);
        assert_eq!(get_logs(), vec!["Method set_last_price doesn't accept the number is neither zero, infinite, subnormal, or NaN"]);
    }

    #[test]
    #[should_panic]
    fn set_neg_infinity_value() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = AveragePrice::default();
        contract.set_last_price(&f64::NEG_INFINITY);
        assert_eq!(get_logs(), vec!["Method set_last_price doesn't accept the number is neither zero, infinite, subnormal, or NaN"]);
    }

    #[test]
    #[should_panic]
    fn set_infinity_value() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = AveragePrice::default();
        contract.set_last_price(&f64::INFINITY);
        assert_eq!(get_logs(), vec!["Method set_last_price doesn't accept the number is neither zero, infinite, subnormal, or NaN"]);
    }

    #[test]
    #[should_panic]
    fn set_zero_value() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = AveragePrice::default();
        contract.set_last_price(&0.0);
        assert_eq!(get_logs(), vec!["Method set_last_price doesn't accept the number is neither zero, infinite, subnormal, or NaN"]);
    }

    #[test]
    #[should_panic]
    fn set_negative_value() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = AveragePrice::default();
        contract.set_last_price(&-1.0);
        assert_eq!(get_logs(), vec!["Method set_last_price doesn't accept the number is neither zero, infinite, subnormal, or NaN"]);
    }

    #[test]
    #[should_panic]
    fn get_average_price_on_empty() {
        let context = get_context(true);
        testing_env!(context);
        let mut contract = AveragePrice::default();
        contract.get_average_price().unwrap();
        assert_eq!(
            get_logs(),
            vec!["No records. Unable to calculate average value."]
        )
    }

    #[test]
    fn get_average() {
        let context = get_context(false);
        testing_env!(context);
        let mut contract = AveragePrice::default();
        contract.set_last_price(&123.0);
        contract.set_last_price(&124.1);
        contract.set_last_price(&123.2345);
        contract.set_last_price(&3453.1284);
        contract.set_last_price(&123.23745);
        assert_eq!(789.34007, contract.get_average_price().unwrap())
    }
}
