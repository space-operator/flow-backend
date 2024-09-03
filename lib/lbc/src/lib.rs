use anyhow::ensure;
use rust_decimal::{Decimal, MathematicalOps};

pub fn lbc_curve(
    interval: Decimal,
    start_price: Decimal,
    mut min_price: Decimal,
    mut max_supply: Decimal,
    time_decay: Option<Decimal>,
) -> Result<TimeDecayExponentialCurve, anyhow::Error> {
    ensure!(
        start_price >= min_price,
        "Max price must be more than min price"
    );
    ensure!(min_price <= Decimal::ZERO, "Min price must be more than 0");
    max_supply *= Decimal::TWO;
    min_price /= Decimal::TWO; // Account for ending with k = 1 instead of k = 0

    // end price = start price / (1 + k0)
    // (1 + k0) (end price) = start price
    // (1 + k0)  = (start price) / (end price)
    // k0  = (start price) / (end price) - 1
    let k0 = start_price / min_price - Decimal::ONE;
    let k1 = Decimal::ONE; // Price should never stop increasing, or it's easy to have a big price drop at the end.

    Ok(TimeDecayExponentialCurve {
        k1,
        k0,
        interval,
        c: Decimal::ONE,
        d: time_decay.unwrap_or_else(|| Decimal::ONE / (k0 - Decimal::ONE).max(Decimal::ONE)),
    })
}

pub struct TimeDecayExponentialCurve {
    pub c: Decimal,
    pub k1: Decimal,
    pub k0: Decimal,
    pub interval: Decimal,
    pub d: Decimal,
}

impl TimeDecayExponentialCurve {
    pub fn price(
        &self,
        time_offset: Decimal,
        base_amount: Decimal,
        target_supply: Decimal,
        amount: Decimal,
        sell: bool,
    ) -> Option<Decimal> {
        if base_amount.eq(&Decimal::ZERO) || target_supply.eq(&Decimal::ZERO) {
            price_exp_initial(
                self.c,
                time_decay_k(self.d, self.k0, self.k1, time_offset, self.interval)?,
                amount,
            )
        } else {
            price_exp(
                time_decay_k(self.d, self.k0, self.k1, time_offset, self.interval)?,
                amount,
                base_amount,
                target_supply,
                sell,
            )
        }
    }
}

fn price_exp(
    k_prec: Decimal,
    amount: Decimal,
    base_amount: Decimal,
    target_supply: Decimal,
    sell: bool,
) -> Option<Decimal> {
    /*
      dR = (R / S^(1 + k)) ((S + dS)^(1 + k) - S^(1 + k))
      dR = (R(S + dS)^(1 + k))/S^(1 + k) - R
      log(dR + R) = log((R(S + dS)^(1 + k))/S^(1 + k))
      log(dR + R) = log((R(S + dS)^(1 + k))) - log(S^(1 + k))
      log(dR + R) = log(R) + (1 + k) log((S + dS)) - (1 + k)log(S)
      log(dR + R) = (1 + k) (log(R(S + dS)) - log(S))
      dR + R = e^(log(R) + (1 + k) (log((S + dS)) - log(S)))
      dR = e^(log(R) + (1 + k) (log((S + dS)) - log(S))) - R
      dR = e^(log(R) + (1 + k) (log((S + dS) / S))) - R

      Edge case: selling where dS = S. Just charge them the full base amount
    */
    let s_plus_ds = if sell {
        target_supply.checked_sub(amount)?
    } else {
        target_supply.checked_add(amount)?
    };
    let one_plus_k_prec = Decimal::ONE.checked_add(k_prec)?;

    // They're killing the curve, so it should cost the full reserves
    if s_plus_ds.eq(&Decimal::ZERO) {
        return Some(base_amount.clone());
    }

    let log1 = base_amount.ln();
    let log2 = s_plus_ds.checked_div(target_supply)?.ln();
    let logs = log1.checked_add(one_plus_k_prec.checked_mul(log2)?)?;
    let exp = logs.exp();

    Some(exp.checked_sub(base_amount)?)
}

fn price_exp_initial(c_prec: Decimal, k_prec: Decimal, amount: Decimal) -> Option<Decimal> {
    // (c dS^(1 + pow/frac))/(1 + pow/frac)
    let one_plus_k_prec = Decimal::ONE.checked_add(k_prec)?;
    c_prec
        .checked_mul(amount.powd(one_plus_k_prec))?
        .checked_div(one_plus_k_prec)
}

fn time_decay_k(
    d: Decimal,
    k0: Decimal,
    k1: Decimal,
    time_offset: Decimal,
    interval: Decimal,
) -> Option<Decimal> {
    let time_multiplier = if time_offset.lt(&interval) {
        let interval_completion = time_offset.checked_div(interval)?;
        interval_completion.ln().checked_mul(d)?.exp()
    } else {
        Decimal::ONE
    };

    Some(k0.checked_sub(k0.checked_sub(k1)?.checked_mul(time_multiplier)?)?)
}
