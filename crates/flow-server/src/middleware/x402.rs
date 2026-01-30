#[derive(Clone)]
pub struct X402Middleware {
    pub(crate) client: x402_kit::facilitator_client::StandardFacilitatorClient,
}

impl X402Middleware {}
