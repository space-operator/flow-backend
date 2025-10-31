use axum::Router;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use std::env;
use x402_axum::{IntoPriceTag, X402Middleware};
use x402_rs::network::{Network, USDCDeployment};
use x402_rs::{address_evm, address_sol};

#[tokio::main]
async fn main() {
    let facilitator_url =
        env::var("FACILITATOR_URL").unwrap_or_else(|_| "https://facilitator.x402.rs".to_string());

    let x402 = X402Middleware::try_from(facilitator_url)
        .unwrap()
        .with_base_url(url::Url::parse("https://localhost:3000/").unwrap());
    let usdc_base_sepolia = USDCDeployment::by_network(Network::BaseSepolia)
        .pay_to(address_evm!("0xBAc675C310721717Cd4A37F6cbeA1F081b1C2a07"));
    let usdc_solana = USDCDeployment::by_network(Network::Solana)
        .pay_to(address_sol!("EGBQqKn968sVv5cQh5Cr72pSTHfxsuzq7o7asqYB5uEV"));

    let app = Router::new().route(
        "/protected-route",
        get(my_handler).layer(
            x402.with_description("Premium API")
                .with_mime_type("application/json")
                .with_price_tag(usdc_solana.amount(0.0001).unwrap())
                .or_price_tag(usdc_base_sepolia.amount(0.0001).unwrap()),
        ),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Can not start server");
    axum::serve(listener, app).await.unwrap();
}

async fn my_handler() -> impl IntoResponse {
    (StatusCode::OK, "This is a VIP content!")
}
