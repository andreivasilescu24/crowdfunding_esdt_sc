#![allow(non_snake_case)]
#![allow(dead_code)]

mod proxy;

use crowdfunding_esdt::endpoints::target;
use multiversx_sc_snippets::imports::*;
use multiversx_sc_snippets::sdk;
use multiversx_sc_snippets::sdk::data::address;
use serde::{Deserialize, Serialize};
use std::os::unix;
use std::{
    io::{Read, Write},
    path::Path,
    thread::sleep,
    time::*,
};

const GATEWAY: &str = sdk::gateway::DEVNET_GATEWAY;
const STATE_FILE: &str = "state.toml";
const DEADLINE_CONTRACT: u64 = 1732516628u64;
const TOKEN_IDENTIFIER: &str = "TTO-281def";
const TARGET_CONTRACT: u128 = 3000000000000000000u128;
const TARGET_UNREACHABLE: u128 = 1000000000000000000000000u128;
const TOKEN_LOW_AMOUNT: u128 = 1000000000000000000u128;

enum AddressType {
    Owner,
    Dan,
    Frank,
}

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
    owner_address: Address,
    dan_address: Address,
    frank_address: Address,
    contract_code: BytesValue,
    state: State,
}

impl ContractInteract {
    async fn new() -> Self {
        let mut interactor = Interactor::new(GATEWAY).await;
        let owner_address = interactor.register_wallet(test_wallets::alice());
        let dan_address = interactor.register_wallet(test_wallets::dan());
        let frank_address = interactor.register_wallet(test_wallets::frank());

        let contract_code = BytesValue::interpret_from(
            "mxsc:../output/crowdfunding-esdt.mxsc.json",
            &InterpreterContext::default(),
        );

        ContractInteract {
            interactor,
            owner_address,
            dan_address,
            frank_address,
            contract_code,
            state: State::load_state(),
        }
    }

    async fn deploy(&mut self, target: u128, deadline: u64, token_identifier: &str) {
        // let target = BigUint::<StaticApi>::from(target);
        // let token_identifier = EgldOrEsdtTokenIdentifier::esdt(token_identifier);

        let new_address = self
            .interactor
            .tx()
            .from(&self.owner_address)
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .init(
                BigUint::from(target),
                deadline,
                TokenIdentifier::from(token_identifier),
            )
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

    async fn upgrade(&mut self, target: u128, deadline: u64, token_identifier: &str) {
        // let target = BigUint::<StaticApi>::from(target);
        // let token_identifier = EgldOrEsdtTokenIdentifier::esdt(token_identifier);

        self.interactor
            .tx()
            .from(&self.owner_address)
            .to(self.state.current_address())
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .upgrade(
                BigUint::from(target),
                deadline,
                TokenIdentifier::from(token_identifier),
            )
            .code(&self.contract_code)
            .code_metadata(CodeMetadata::UPGRADEABLE)
            .prepare_async()
            .run()
            .await;

        println!("upgrade completed");
    }

    async fn fund_egld(&mut self, token_amount: u128) {
        ////////////
        // let token_id = String::new();
        // let token_nonce = 0u64;
        // let token_amount = BigUint::<StaticApi>::from(0u128);

        let response = self
            .interactor
            .tx()
            .from(&self.owner_address)
            .to(self.state.current_address())
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .fund()
            .egld(BigUint::from(token_amount))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn fund(
        &mut self,
        token_id: &str,
        token_nonce: u64,
        token_amount: u128,
        address_type: AddressType,
    ) {
        let sender_addr: &Address = match address_type {
            AddressType::Dan => &self.dan_address,
            AddressType::Frank => &self.frank_address,
            AddressType::Owner => &self.owner_address,
        };
        let response = self
            .interactor
            .tx()
            .from(sender_addr)
            .to(self.state.current_address())
            // .egld(100000000000000000)
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .fund()
            .payment((
                TokenIdentifier::from(token_id),
                token_nonce,
                BigUint::from(token_amount),
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

    async fn claim(&mut self, address_type: AddressType) {
        let sender_addr: &Address = match address_type {
            AddressType::Dan => &self.dan_address,
            AddressType::Frank => &self.frank_address,
            AddressType::Owner => &self.owner_address,
        };

        let result_value = self
            .interactor
            .tx()
            .from(sender_addr)
            .to(self.state.current_address())
            .typed(proxy::CrowdfundingProxy)
            .claim()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
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
        self.interactor
            .tx()
            .from(&self.owner_address)
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .init(target, deadline, TokenIdentifier::from(token_identifier))
            .code(&self.contract_code)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
    }

    async fn claim_fail(&mut self, expected_result: ExpectError<'_>, address_type: AddressType) {
        let sender_address = match address_type {
            AddressType::Dan => &self.dan_address,
            AddressType::Frank => &self.frank_address,
            AddressType::Owner => &self.owner_address,
        };
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

    async fn get_curr_funds(&mut self) -> BigUint<StaticApi> {
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
        BigUint::from(result_value)
    }

    async fn get_deposit(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::CrowdfundingProxy)
            .deposit(&self.dan_address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }
}

fn get_unix_timestamp() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let unix_timestamp = since_the_epoch.as_secs();
    unix_timestamp
}

fn calculate_reachable_deadline() -> u64 {
    get_unix_timestamp() + 25
}

fn wait_past_deadline(deadline: u64) {
    while get_unix_timestamp() <= deadline {
        sleep(Duration::from_secs(1));
    }
}

#[tokio::test]
async fn test_deploy() {
    let mut interact = ContractInteract::new().await;
    let deadline = DEADLINE_CONTRACT;
    let token_identifier = TOKEN_IDENTIFIER;
    interact
        .deploy(TARGET_CONTRACT, deadline, token_identifier)
        .await;
}

#[tokio::test]
async fn test_claim_deadline_fail() {
    let mut interact = ContractInteract::new().await;
    interact
        .upgrade(TARGET_CONTRACT, DEADLINE_CONTRACT, TOKEN_IDENTIFIER)
        .await;
    interact
        .fund(TOKEN_IDENTIFIER, 0, 1000000000000000000, AddressType::Dan)
        .await;

    interact
        .claim_fail(
            ExpectError(4, "cannot claim before deadline"),
            AddressType::Dan,
        )
        .await;
}

#[tokio::test]
async fn test_claim_fail_user_not_owner() {
    let mut interact = ContractInteract::new().await;

    let deadline = calculate_reachable_deadline();

    interact
        .upgrade(TARGET_CONTRACT, deadline, TOKEN_IDENTIFIER)
        .await;
    interact
        .fund(TOKEN_IDENTIFIER, 0, TARGET_CONTRACT, AddressType::Dan)
        .await;

    wait_past_deadline(deadline);

    interact
        .claim_fail(
            ExpectError(4, "only owner can claim successful funding"),
            AddressType::Dan,
        )
        .await;
}

#[tokio::test]
async fn test_claim_owner() {
    let mut interact = ContractInteract::new().await;

    let deadline = calculate_reachable_deadline();
    interact
        .upgrade(TARGET_CONTRACT, deadline, TOKEN_IDENTIFIER)
        .await;

    interact
        .fund(TOKEN_IDENTIFIER, 0, TARGET_CONTRACT, AddressType::Dan)
        .await;

    wait_past_deadline(deadline);

    interact.claim(AddressType::Owner).await;
}

#[tokio::test]
async fn test_target_not_achieved_user_claim() {
    let mut interact = ContractInteract::new().await;

    let deadline = calculate_reachable_deadline();

    interact
        .upgrade(TARGET_UNREACHABLE, deadline, TOKEN_IDENTIFIER)
        .await;

    interact
        .fund(TOKEN_IDENTIFIER, 0, TOKEN_LOW_AMOUNT, AddressType::Dan)
        .await;

    wait_past_deadline(deadline);

    interact.claim(AddressType::Dan).await;
}

#[tokio::test]
async fn test_already_claimed_funds() {
    let mut interact = ContractInteract::new().await;

    let deadline = calculate_reachable_deadline();

    interact
        .upgrade(TARGET_UNREACHABLE, deadline, TOKEN_IDENTIFIER)
        .await;

    interact
        .fund(TOKEN_IDENTIFIER, 0, TOKEN_LOW_AMOUNT, AddressType::Dan)
        .await;

    wait_past_deadline(deadline);

    interact.claim(AddressType::Dan).await;
    sleep(Duration::from_secs(5));
    interact
        .claim_fail(ExpectError(4, "insufficient funds"), AddressType::Dan)
        .await;
}

#[tokio::test]
async fn test_get_current_funds() {
    let mut interact = ContractInteract::new().await;
    interact.get_current_funds().await;
}

#[tokio::test]
async fn test_get_target() {
    let mut interact = ContractInteract::new().await;
    interact.get_target().await;
}

#[tokio::test]
async fn test_get_deadline() {
    let mut interact = ContractInteract::new().await;
    interact.get_deadline().await;
}

#[tokio::test]
async fn test_get_deposit() {
    let mut interact = ContractInteract::new().await;
    interact.get_deposit().await;
}

// #[tokio::test]
// async fn test_deploy_bad_parameters() {
//     let mut interact = ContractInteract::new().await;
//     let target_fail = BigUint::<StaticApi>::from(0u128);
//     let target_pass = BigUint::<StaticApi>::from(5u128);
//     let deadline_fail = 1722516628u64;
//     let deadline_pass = 1822516628u64;
//     let token_id_pass = "EGLD-123456";
//     //let token_id_incorrect = TokenIdentifier::from("ESVT-1234566653");
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

#[tokio::test]
async fn test_fund_pass() {
    let mut interact = ContractInteract::new().await;
    // let token_id1 = "EGLD";
    let token_id2 = "BSK-476470";
    let token_nonce = 0u64;
    let token_amount = 500000000000000000u128;

    let target = 5u128;
    let deadline = 1732516628u64;
    interact.upgrade(target, deadline, token_id2).await;
    interact
        .fund(token_id2, token_nonce, token_amount, AddressType::Dan)
        .await;

    assert_eq!(1, 1)
}
