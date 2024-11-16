#[cfg(test)]
mod tests {
    use flow::{flow_run_events::event_channel, FlowGraph};
    use flow_lib::{
        config::client::ClientConfig,
        // solana::{Keypair, Wallet},
        FlowConfig,
    };

    use serde::Deserialize;
    use value::Value;

    #[derive(Deserialize)]
    struct TestFile {
        flow: ClientConfig,
    }

    #[tokio::test]
    async fn test_flow_run() {
        const KEYPAIR: &str =
            "3LUpzbebV5SCftt8CPmicbKxNtQhtJegEz4n8s6LBf3b1s4yfjLapgJhbMERhP73xLmWEP2XJ2Rz7Y3TFiYgTpXv";
        // let wallet = Wallet::Keypair(Keypair::from_base58_string(KEYPAIR));

        let json = include_str!("../../../../test_files/create_token.json");
        let flow_config = FlowConfig::new(serde_json::from_str::<TestFile>(json).unwrap().flow);
        // dbg!(&flow_config);

        // create a token flow inputs without a default value
        let inputs: value::Map = {
            let mut map = value::Map::new();
            map.insert(
                "fee_payer".to_string(),
                Value::new_keypair_bs58(KEYPAIR).unwrap(),
            );
            map.insert(
                "mint_authority".to_string(),
                Value::new_keypair_bs58(KEYPAIR).unwrap(),
            );
            map
        };

        // to display the available commands
        // let c = flow::context::CommandFactory::new();
        // let keys = c.natives.keys().collect::<Vec<_>>();
        // dbg!(keys);

        let mut flow = FlowGraph::from_cfg(flow_config, <_>::default(), None)
            .await
            .unwrap();

        let (tx, _rx) = event_channel();
        let res = flow
            .run(
                tx,
                <_>::default(),
                inputs,
                <_>::default(),
                <_>::default(),
                <_>::default(),
            )
            .await;
        dbg!(&res.output);

        // get the mint account from the flow output node
        let mint = res.output.get("mint_account").unwrap();
        dbg!(&mint);
    }
}
