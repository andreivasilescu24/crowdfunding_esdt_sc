#![allow(non_snake_case)]
#![allow(dead_code)]

mod proxy;

use crowdfunding_esdt::endpoints::target;
use multiversx_sc_snippets::imports::*;
use multiversx_sc_snippets::sdk;
use multiversx_sc_snippets::sdk::data::address;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
};

const GATEWAY: &str = sdk::gateway::DEVNET_GATEWAY;
const STATE_FILE: &str = "state.toml";

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut args = std::env::args();
    let _ = args.next();
    // let cmd = args.next().expect("at least one argument required");
    // let mut interact = ContractInteract::new().await;
    // match cmd.as_str() {
    //     "deploy" => interact.deploy().await,
    //     "fund" => interact.fund().await,
    //     "status" => interact.status().await,
    //     "getCurrentFunds" => interact.get_current_funds().await,
    //     "claim" => interact.claim().await,
    //     "getTarget" => interact.target().await,
    //     "getDeadline" => interact.deadline().await,
    //     "getDeposit" => interact.deposit().await,
    //     "getCrowdfundingTokenIdentifier" => interact.cf_token_identifier().await,
    //     _ => panic!("unknown command: {}", &cmd),
    // }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    contract_address: Option<Bech32Address>,
}

impl State {
    // Deserializes state from file
    pub fn load_state() -> Self {
        if Path::new(STATE_FILE).exists() {
            let mut file = std::fs::File::open(STATE_FILE).unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            toml::from_str(&content).unwrap()
        } else {
            Self::default()
        }
    }

    /// Sets the contract address
    pub fn set_address(&mut self, address: Bech32Address) {
        self.contract_address = Some(address);
    }

    /// Returns the contract address
    pub fn current_address(&self) -> &Bech32Address {
        self.contract_address
            .as_ref()
            .expect("no known contract, deploy first")
    }
}

impl Drop for State {
    // Serializes state to file
    fn drop(&mut self) {
        let mut file = std::fs::File::create(STATE_FILE).unwrap();
        file.write_all(toml::to_string(self).unwrap().as_bytes())
            .unwrap();
    }
}

struct ContractInteract {
    interactor: Interactor,
    wallet_address: Address,
    user_address: Address,
    contract_code: BytesValue,
    state: State,
}

impl ContractInteract {
    async fn new() -> Self {
        let mut interactor = Interactor::new(GATEWAY).await;
        let wallet_address = interactor.register_wallet(test_wallets::alice());
        let user_address = interactor.register_wallet(test_wallets::bob());

        let contract_code = BytesValue::interpret_from(
            "mxsc:../output/crowdfunding-esdt.mxsc.json",
            &InterpreterContext::default(),
        );

        ContractInteract {
            interactor,
            wallet_address,
            user_address,
            contract_code,
            state: State::load_state(),
        }
    }

    async fn deploy(&mut self, target: BigUint<StaticApi>, deadline: u64, token_identifier: &str) {
        // let target = BigUint::<StaticApi>::from(target);
        // let token_identifier = EgldOrEsdtTokenIdentifier::esdt(token_identifier);

        let new_address = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .init(target, deadline, TokenIdentifier::from(token_identifier))
            .code(&self.contract_code)
            .returns(ReturnsNewAddress)
            .prepare_async()
            .run()
            .await;
        let new_address_bech32 = bech32::encode(&new_address);
        self.state.set_address(Bech32Address::from_bech32_string(
            new_address_bech32.clone(),
        ));

        println!("new address: {new_address_bech32}");
    }

    async fn fund(&mut self, token_id: &str, token_nonce: u64, token_amount: u128) {
        // let token_id = String::new();
        // let token_nonce = 0u64;
        // let token_amount = BigUint::<StaticApi>::from(0u128);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            // .egld(100000000000000000)
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .fund()
            .payment((
                TokenIdentifier::from(token_id),
                token_nonce,
                BigUint::<StaticApi>::from(token_amount),
            ))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn status(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::CrowdfundingProxy)
            .status()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn get_current_funds(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::CrowdfundingProxy)
            .get_current_funds()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn claim(&mut self) {
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .claim()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn target(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::CrowdfundingProxy)
            .target()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn deadline(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::CrowdfundingProxy)
            .deadline()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn deposit(&mut self) {
        let donor = bech32::decode("");

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::CrowdfundingProxy)
            .deposit(donor)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn cf_token_identifier(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::CrowdfundingProxy)
            .cf_token_identifier()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn deploy_bad_parameters(
        &mut self,
        target: BigUint<StaticApi>,
        deadline: u64,
        token_identifier: &str,
        expected_result: ExpectError<'_>,
    ) {
        // let target = BigUint::<StaticApi>::from(target);
        // let token_identifier = EgldOrEsdtTokenIdentifier::esdt(token_identifier);

        self.interactor
            .tx()
            .from(&self.wallet_address)
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .init(target, deadline, TokenIdentifier::from(token_identifier))
            .code(&self.contract_code)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
        // let new_address_bech32 = bech32::encode(&new_address.0);
        // self.state.set_address(Bech32Address::from_bech32_string(
        //     new_address_bech32.clone(),
        // ));

        // println!("new address: {new_address_bech32}");
    }

    async fn claim_fail(&mut self, expected_result: ExpectError<'_>, sender_address: &Address) {
        self.interactor
            .tx()
            .from(sender_address)
            .to(self.state.current_address())
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .claim()
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
    }

    async fn get_deadline(&mut self) -> u64 {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::CrowdfundingProxy)
            .deadline()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
        result_value
    }

    async fn get_target(&mut self) -> BigUint<StaticApi> {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::CrowdfundingProxy)
            .target()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
        BigUint::from(result_value)
    }
}

#[tokio::test]
async fn test_deploy() {
    let mut interact = ContractInteract::new().await;
    let target = BigUint::<StaticApi>::from(5u128);
    let deadline = 1732516628u64;
    let token_identifier = "EGLD";
    interact.deploy(target, deadline, token_identifier).await;
}

#[tokio::test]
async fn test_claim_fail() {
    let mut interact = ContractInteract::new().await;
    let owner_address = &interact.wallet_address;
    let user_address = &interact.user_address;

    interact.fund()
}

#[tokio::test]
async fn test_claim_fail() {}
// #[tokio::test]
// async fn test_deploy_bad_parameters() {
//     let mut interact = ContractInteract::new().await;
//     let target_fail = BigUint::<StaticApi>::from(0u128);
//     let target_pass = BigUint::<StaticApi>::from(5u128);
//     let deadline_fail = 1722516628u64;
//     let deadline_pass = 1822516628u64;
//     let token_id_pass = TokenIdentifier::from_esdt_bytes(&b"EGLD-123456"[..]);
//     let token_id_incorrect = TokenIdentifier::from("ESVT-1234566653");
//     // let token_id_nft = TokenIdentifier::from_esdt_bytes(&b"TEST-123456-01"[..]);
//     interact
//         .deploy_bad_parameters(
//             target_fail,
//             deadline_pass,
//             token_id_pass,
//             ExpectError(4, "Target must be more than 0"),
//         )
//         .await;
// }
