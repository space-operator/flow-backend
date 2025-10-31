use x402_rs::{network::Network, types::PaymentRequirements};

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

fn create_exact_paypemt_requirements(network: Network) -> PaymentRequirements {
    PaymentRequirements {
        scheme: x402_rs::types::Scheme::Exact,
        network,
        max_amount_required: (),
        resource: (),
        description: (),
        mime_type: (),
        output_schema: (),
        pay_to: (),
        max_timeout_seconds: (),
        asset: (),
        extra: (),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
