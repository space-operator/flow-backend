use anyhow::ensure;
use rust_decimal::{prelude::FromPrimitive, prelude::ToPrimitive, Decimal, MathematicalOps};

pub fn lbc_curve(
    interval: Decimal,
    start_price: Decimal,
    mut min_price: Decimal,
    max_price: Decimal,
    mut max_supply: Decimal,
    time_decay: Option<Decimal>,
) -> Result<TimeDecayExponentialCurve, anyhow::Error> {
    ensure!(
        start_price >= min_price,
        "Max price must be more than min price"
    );
    ensure!(min_price > Decimal::ZERO, "Min price must be more than 0");
    max_supply *= Decimal::TWO;
    min_price /= Decimal::TWO; // Account for ending with k = 1 instead of k = 0

    ensure!(
        max_price > start_price,
        "Max price must be more than start price"
    );

    let k0 = start_price;
    let k1 = max_price;

    Ok(TimeDecayExponentialCurve {
        k1,
        k0,
        interval,
        c: Decimal::new(1, 2),
        d: time_decay.unwrap_or_else(|| Decimal::new(5, 2)),
        max_price,
        min_price,
        last_purchase_time: None,
        price_increase_factor: Decimal::new(5, 4),
        decay_window: std::time::Duration::from_secs(3600),
        recent_purchases: Vec::new(),
        max_recent_purchases: 100,
        decay_factor: Decimal::new(99, 2),
        decay_interval: std::time::Duration::from_secs(3600), // 1 hour, adjust as needed
        minutely_decay_percentage: Decimal::new(50000000, 9),
        last_calculated_price: start_price, // Initial price
        last_price_calculation_time: std::time::Instant::now(), // Initial time
    })
}

pub struct TimeDecayExponentialCurve {
    pub c: Decimal,         // Constant that affects the overall scale of the curve
    pub k1: Decimal,        // Final value of k (the exponent) after time decay
    pub k0: Decimal,        // Initial value of k before time decay
    pub interval: Decimal,  // Time interval over which the decay occurs
    pub d: Decimal,         // Decay factor that determines how quickly k changes from k0 to k1
    pub max_price: Decimal, // Maximum price allowed for the curve
    pub min_price: Decimal, // Minimum price allowed for the curve
    pub last_purchase_time: Option<std::time::Instant>, // Timestamp of the most recent purchase
    pub price_increase_factor: Decimal, // Factor by which price increases due to recent purchases
    pub decay_window: std::time::Duration, // Time window for considering recent purchases
    pub recent_purchases: Vec<std::time::Instant>, // List of timestamps for recent purchases
    pub max_recent_purchases: usize, // Maximum number of recent purchases to track
    pub decay_factor: Decimal, 
    pub decay_interval: std::time::Duration,
    pub minutely_decay_percentage: Decimal, 
    pub last_calculated_price: Decimal, 
    pub last_price_calculation_time: std::time::Instant, 
}

impl TimeDecayExponentialCurve {
    pub fn price(
        &mut self,
        time_offset: Decimal,
        base_amount: Decimal,
        target_supply: Decimal,
        amount: Decimal,
    ) -> Option<Decimal> {
        let now = std::time::Instant::now();
        let time_since_last_calculation = now.duration_since(self.last_price_calculation_time);
        let minutes_elapsed = time_since_last_calculation.as_secs_f64() / 60.0;

        // If it's the first buy (base_amount and target_supply are zero), return k0
        // if target_supply.is_zero() && amount > Decimal::ZERO {
        //     return Some(self.k0);
        // }

        // Calculate the base price using the bonding curve
        let k = time_decay_k(self.d, self.k0, self.k1, time_offset, self.interval)?;
        let base_price = price_exp(k, amount.max(Decimal::ONE), base_amount, target_supply)?;

        let mut price = base_price;
        // println!("Base price: {}", base_price);

        if amount > Decimal::ZERO {
            // Price increase logic for purchases
            self.recent_purchases.push(now);
            if self.recent_purchases.len() > self.max_recent_purchases {
                self.recent_purchases.remove(0);
            }

            self.recent_purchases
                .retain(|&time| now.duration_since(time) < self.decay_window);
            let purchase_count = self.recent_purchases.len() as f64;
            let max_increase = self.price_increase_factor.to_f64().unwrap_or(0.005);
            // println!("Max increase: {}", max_increase);

            let total_increase = 1.0 + (purchase_count * max_increase).min(max_increase);
            // println!("Total increase: {}", total_increase);

            price *= Decimal::from_f64(total_increase).unwrap_or(Decimal::ONE);
            // println!("Price: {}", price);
            self.last_purchase_time = Some(now);
        } else {
            // Price decay logic when no purchase is made ... like an additional decay to simulate selling...
            let decay_factor = Decimal::ONE
                - (self.minutely_decay_percentage
                    * Decimal::new(3000000, 0)
                    * Decimal::from_f64(minutes_elapsed).unwrap_or(Decimal::ZERO))
                .min(Decimal::new(5, 1));
            let decay_factor = decay_factor.max(Decimal::new(5, 1));
            price *= decay_factor;
        }

        // Ensure the price doesn't drop below a minimum threshold
        price = price.max(self.min_price);

        // Apply max price limit
        price = price.min(self.max_price);

        // Update last calculated price and time
        self.last_calculated_price = price;
        self.last_price_calculation_time = now;

        Some(price)
    }
}

fn price_exp(
    k_prec: Decimal,
    amount: Decimal,
    base_amount: Decimal,
    target_supply: Decimal,
) -> Option<Decimal> {
    let s_plus_ds = target_supply.checked_add(amount)?;
    let one_plus_k = Decimal::ONE.checked_add(k_prec)?;

    // Use checked operations to avoid panics
    s_plus_ds.checked_powd(one_plus_k).and_then(|numerator| {
        target_supply
            .checked_powd(one_plus_k)
            .and_then(|denominator| numerator.checked_div(denominator))
            .and_then(|ratio| ratio.checked_sub(Decimal::ONE))
            .and_then(|diff| base_amount.checked_mul(diff))
            .and_then(|total| total.checked_div(amount))
    })
}

fn price_exp_initial(c_prec: Decimal, k_prec: Decimal, amount: Decimal) -> Option<Decimal> {
    // c * (dS^(1 + k)) / (1 + k)
    let one_plus_k_prec = Decimal::ONE.checked_add(k_prec)?;
    c_prec
        .checked_mul(amount.powd(one_plus_k_prec))?
        .checked_div(one_plus_k_prec)
}

fn time_decay_k(
    d: Decimal,           // Decay factor that determines how quickly k changes from k0 to k1
    k0: Decimal,          // Initial value of k before time decay
    k1: Decimal,          // Final value of k after time decay
    time_offset: Decimal, // Current time offset from the start of the curve
    interval: Decimal,    // Time interval over which the decay occurs
) -> Option<Decimal> {
    let time_multiplier = if time_offset.lt(&interval) {
        let interval_completion = time_offset.checked_div(interval)?;
        Decimal::ONE - interval_completion.powd(d)
    } else {
        Decimal::ZERO
    };

    // Interpolate between k0 and k1 based on the time_multiplier
    Some(k0 + (k1 - k0) * (Decimal::ONE - time_multiplier))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use rust_decimal::prelude::ToPrimitive;
    use rust_decimal_macros::dec;
    use textplots::{Chart, Plot, Shape};

    #[test]
    fn test_simulate_curve_over_time() {
        let simulation_duration = 7 * 24 * 3600; // 7 days in seconds
        let interval = dec!(604800); // 1 hour in seconds

        let start_price = dec!(0.2);
        let min_price = dec!(0.1);
        let max_price = dec!(1);
        let max_supply = dec!(5000);
        let time_decay = Some(dec!(0.1));

        let mut curve = lbc_curve(
            interval,
            start_price,
            min_price,
            max_price,
            max_supply,
            time_decay,
        )
        .unwrap();

        let mut base_amount = dec!(0);
        let mut target_supply = dec!(0);
        let six_hours = 6 * 3600; // 6 hours in seconds
        let thirty_six_hours = 36 * 3600; // 36 hours in seconds

        println!("Time (hours) | Price | Action | Amount | Base Amount | Target Supply");
        println!("-------------|-------|--------|--------|-------------|---------------");

        let mut rng = rand::thread_rng();
        let mut price_data = Vec::new();
        let mut time = 0;
        let mut last_price = dec!(0);
        let mut total_tokens_bought = dec!(0);
        let mut total_amount_raised = dec!(0);

        while total_tokens_bought < max_supply && time < simulation_duration {
            let time_offset = Decimal::from(time);
            let hours = time_offset / dec!(3600);

            let (action, amount) = if time < six_hours {
                // First 6 hours: buy 1 token every 10-15 minutes
                ("Buy", Decimal::from(1))
            } else if time < thirty_six_hours {
                // Between 6 and 36 hours: 50% chance to buy 1 token every 30-60 minutes
                if time - six_hours >= 1800 {
                    if rng.gen_bool(0.9) {
                        ("Buy", Decimal::from(1))
                    } else {
                        ("Wait", Decimal::ZERO)
                    }
                } else {
                    ("Wait", Decimal::ZERO)
                }
            } else {
                // After 36 hours: buy 1 token every 2-2.5 hours
                if time - thirty_six_hours >= 7200 {
                    if rng.gen_bool(0.2) {
                        ("Buy", Decimal::from(1))
                    } else {
                        ("Wait", Decimal::ZERO)
                    }
                } else {
                    ("Wait", Decimal::ZERO)
                }
            };

            // Adjust wait time to match the intended intervals
            let wait_time = if time < six_hours {
                rng.gen_range(600..=900) // 10-15 minutes
            } else if time < thirty_six_hours {
                rng.gen_range(1800..=3600) // 30-60 minutes
            } else {
                rng.gen_range(7200..=9000) // 120-150 minutes (2-2.5 hours)
            };
            time += wait_time;

            let current_price = curve
                .price(time_offset, base_amount, target_supply, amount)
                .unwrap_or_else(|| {
                    println!("Price calculation failed at time {}", time_offset);
                    Decimal::ZERO
                });
            last_price = current_price;

            if action == "Buy" {
                let amount_raised = current_price * amount;
                base_amount += amount_raised;
                target_supply += amount;
                total_tokens_bought += amount;
                total_amount_raised += amount_raised; // Add this line
            }

            println!(
                "{:12.2} | {:5.12} | {:5} | {:6.2} | {:11.2} | {:13.2}",
                hours, current_price, action, amount, base_amount, target_supply
            );

            // Update this line to use time in hours directly
            price_data.push((time as f32 / 3600.0, current_price.to_f32().unwrap()));
        }

        // Update the Chart::new() call
        let max_time_hours = simulation_duration as f32 / 3600.0;
        Chart::new(240, 60, 0.0, max_time_hours)
            .lineplot(&Shape::Lines(&price_data))
            .nice();

        println!(
            "Simulation ended at {} hours",
            Decimal::from(time) / dec!(3600)
        );
        println!("Final price: {}", last_price);
        println!("Final base amount: {}", base_amount);
        println!("Final target supply: {}", target_supply);
        println!("Total tokens bought: {}", total_tokens_bought);
        println!("Total amount raised: {}", total_amount_raised); // Add this line
    }
}
