#![allow(non_snake_case)]
#![allow(dead_code)]

mod proxy;

use crowdfunding_esdt::endpoints::status;
use crowdfunding_esdt::endpoints::target;
use multiversx_sc_snippets::imports::*;
use multiversx_sc_snippets::sdk;
use multiversx_sc_snippets::sdk::data::address;
use reqwest::Client;
use serde::de;
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

use std::time::{SystemTime, UNIX_EPOCH};

const TOKEN_ID_EGLD: &str = "EGLD";
const TOKEN_ID_TTO: &str = "TTO-281def";
const TOKEN_ID_WRONG_TOKEN: &str = "BSK-476470";
const TOKEN_NONCE: u64 = 0;
const TOKEN_AMOUNT: u128 = 500000000000000000;
const TARGET: u128 = 5;
const DEADLINE: u64 = 1732516628;
const PAST_DEADLINE: u64 = 1722597975;

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
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let unix_timestamp = since_the_epoch.as_secs();

    println!("Current Unix timestamp: {}", unix_timestamp);
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

    async fn deploy_succ(&mut self, target: u128, deadline: u64, token_identifier: &str) {
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

    async fn fund_egld_succ(&mut self, token_amount: u128) {
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

    async fn fund_succ(
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

    async fn fund_egld_failed(&mut self, token_amount: u128, expected_result: ExpectError<'_>) {
        let response = self
            .interactor
            .tx()
            .from(&self.owner_address)
            .to(self.state.current_address())
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .fund()
            .egld(BigUint::from(token_amount))
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn fund_failed(
        &mut self,
        token_id: &str,
        token_nonce: u64,
        token_amount: u128,
        expected_result: ExpectError<'_>,
    ) {
        let response = self
            .interactor
            .tx()
            .from(&self.owner_address)
            .to(self.state.current_address())
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .fund()
            .payment((
                TokenIdentifier::from(token_id),
                token_nonce,
                BigUint::from(token_amount),
            ))
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn get_current_funds(&mut self) -> BigUint<StaticApi> {
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

    async fn claim_succ(&mut self, address_type: AddressType) {
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

    async fn depositDan(&mut self) -> BigUint<StaticApi> {
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
        BigUint::from(result_value)
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
        target: u128,
        deadline: u64,
        token_identifier: &str,
        expected_result: ExpectError<'_>,
    ) {
        self.interactor
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
            .deposit(&self.frank_address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn status(&mut self) -> proxy::Status {
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
        result_value
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
        .deploy_succ(TARGET_CONTRACT, deadline, token_identifier)
        .await;
}

#[tokio::test]
async fn test_deploy_bad_parameters() {
    let mut interact = ContractInteract::new().await;
    let deadline = get_unix_timestamp() - 20;

    interact
        .deploy_bad_parameters(
            TARGET_CONTRACT,
            deadline,
            TOKEN_IDENTIFIER,
            ExpectError(4, "Deadline can't be in the past"),
        )
        .await;

    interact
        .deploy_bad_parameters(
            0,
            DEADLINE_CONTRACT,
            TOKEN_IDENTIFIER,
            ExpectError(4, "Target must be more than 0"),
        )
        .await;

    interact
        .deploy_bad_parameters(
            TARGET_CONTRACT,
            DEADLINE_CONTRACT,
            "TTO-12312313123131",
            ExpectError(4, "Invalid token provided"),
        )
        .await;
}

#[tokio::test]
async fn test_claim_deadline_fail() {
    let mut interact = ContractInteract::new().await;
    interact
        .upgrade(TARGET_CONTRACT, DEADLINE_CONTRACT, TOKEN_IDENTIFIER)
        .await;
    interact
        .fund_succ(TOKEN_IDENTIFIER, 0, TOKEN_LOW_AMOUNT, AddressType::Dan)
        .await;

    interact
        .claim_fail(
            ExpectError(4, "cannot claim before deadline"),
            AddressType::Dan,
        )
        .await;

    interact
        .claim_fail(
            ExpectError(4, "cannot claim before deadline"),
            AddressType::Owner,
        )
        .await;
}

#[tokio::test]
async fn test_claim_fail_user_not_owner() {
    let mut interact = ContractInteract::new().await;

    let deadline = get_unix_timestamp() + 25;

    interact
        .upgrade(TARGET_CONTRACT, deadline, TOKEN_IDENTIFIER)
        .await;
    interact
        .fund_succ(TOKEN_IDENTIFIER, 0, TARGET_CONTRACT, AddressType::Dan)
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

    let deadline = get_unix_timestamp() + 25;
    interact
        .deploy_succ(TARGET_CONTRACT, deadline, TOKEN_IDENTIFIER)
        .await;

    interact
        .fund_succ(TOKEN_IDENTIFIER, 0, TARGET_CONTRACT, AddressType::Dan)
        .await;

    wait_past_deadline(deadline);

    interact.claim_succ(AddressType::Owner).await;
    interact.claim_succ(AddressType::Owner).await;
}

#[tokio::test]
async fn test_target_not_achieved_user_claim() {
    let mut interact = ContractInteract::new().await;

    let deadline = get_unix_timestamp() + 25;

    interact
        .upgrade(TARGET_UNREACHABLE, deadline, TOKEN_IDENTIFIER)
        .await;

    interact
        .fund_succ(TOKEN_IDENTIFIER, 0, TOKEN_LOW_AMOUNT, AddressType::Frank)
        .await;

    wait_past_deadline(deadline);

    interact.claim_succ(AddressType::Frank).await;
}

#[tokio::test]
async fn test_multiple_funds() {
    let mut interact = ContractInteract::new().await;

    let deadline = get_unix_timestamp() + 30;

    interact
        .deploy_succ(TARGET_UNREACHABLE, deadline, TOKEN_IDENTIFIER)
        .await;

    for _ in 0..2 {
        interact
            .fund_succ(TOKEN_IDENTIFIER, 0, TOKEN_LOW_AMOUNT, AddressType::Dan)
            .await;
        interact
            .fund_succ(TOKEN_IDENTIFIER, 0, TOKEN_LOW_AMOUNT, AddressType::Frank)
            .await;
    }
    wait_past_deadline(deadline);

    interact.claim_succ(AddressType::Dan).await;
    interact.claim_succ(AddressType::Frank).await;
}

#[tokio::test]
async fn test_status_funding() {
    let mut interact = ContractInteract::new().await;

    interact
        .deploy_succ(TARGET_UNREACHABLE, DEADLINE_CONTRACT, TOKEN_IDENTIFIER)
        .await;

    let status = interact.status().await;
    assert_eq!(status, proxy::Status::FundingPeriod);
}

#[tokio::test]
async fn test_status_succ() {
    let mut interact = ContractInteract::new().await;
    let deadline = get_unix_timestamp() + 20;
    interact
        .deploy_succ(TARGET_CONTRACT, deadline, TOKEN_IDENTIFIER)
        .await;
    interact
        .fund_succ(
            TOKEN_IDENTIFIER,
            0,
            TARGET_CONTRACT + 10000000000,
            AddressType::Dan,
        )
        .await;

    wait_past_deadline(deadline + 40);

    let status = interact.status().await;
    assert_eq!(status, proxy::Status::Successful);
}

#[tokio::test]
async fn test_status_failed() {
    let mut interact = ContractInteract::new().await;
    let deadline = get_unix_timestamp() + 20;
    interact
        .deploy_succ(TARGET_UNREACHABLE, deadline, TOKEN_IDENTIFIER)
        .await;
    interact
        .fund_succ(TOKEN_IDENTIFIER, 0, TARGET_CONTRACT, AddressType::Dan)
        .await;
    wait_past_deadline(deadline + 40);

    let status = interact.status().await;
    assert_eq!(status, proxy::Status::Failed);
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

// DEPLOY TEST
#[tokio::test]
async fn test_deploy_egld() {
    let mut interact = ContractInteract::new().await;
    let token_nonce = 0u64;
    let token_amount = 500000000000000000u128;

    let target = 5u128;
    let deadline = 1732516628u64;
    interact.upgrade(target, deadline, TOKEN_IDENTIFIER).await;
    interact
        .fund_succ(
            TOKEN_IDENTIFIER,
            token_nonce,
            token_amount,
            AddressType::Dan,
        )
        .await;

    interact
        .deploy_succ(target, get_unix_timestamp() + 10, TOKEN_ID_EGLD)
        .await;
}

#[tokio::test]
async fn test_deploy_token() {
    let mut interact = ContractInteract::new().await;
    let target = 5u128;
    interact
        .deploy_succ(target, get_unix_timestamp() + 10, TOKEN_ID_TTO)
        .await;
}

// FUND EGLD TESTS
#[tokio::test]
async fn fund_egld() {
    let mut interact = ContractInteract::new().await;

    interact.upgrade(TARGET, DEADLINE, TOKEN_ID_EGLD).await;
    interact.fund_egld_succ(TOKEN_AMOUNT).await;
}

#[tokio::test]
async fn fund_egld_wrong_token() {
    let mut interact = ContractInteract::new().await;

    interact.upgrade(TARGET, DEADLINE, TOKEN_ID_TTO).await;
    interact
        .fund_egld_failed(TOKEN_AMOUNT, ExpectError(4, "wrong token"))
        .await;
}

#[tokio::test]
async fn fund_egld_past_deadline() {
    let mut interact = ContractInteract::new().await;
    let deadline = get_unix_timestamp() + 20;

    interact.upgrade(TARGET, deadline, TOKEN_ID_EGLD).await;

    wait_past_deadline(deadline);

    interact
        .fund_egld_failed(TOKEN_AMOUNT, ExpectError(4, "cannot fund after deadline"))
        .await;
}

// FUND ESDTs TESTS
#[tokio::test]
async fn fund_token() {
    let mut interact = ContractInteract::new().await;

    interact.upgrade(TARGET, DEADLINE, TOKEN_ID_TTO).await;
    interact
        .fund_succ(TOKEN_ID_TTO, TOKEN_NONCE, TOKEN_AMOUNT, AddressType::Dan)
        .await;
}

#[tokio::test]
async fn fund_wrong_token() {
    let mut interact = ContractInteract::new().await;

    interact.upgrade(TARGET, DEADLINE, TOKEN_ID_TTO).await;
    interact
        .fund_failed(
            TOKEN_ID_WRONG_TOKEN,
            TOKEN_NONCE,
            TOKEN_AMOUNT,
            ExpectError(4, "wrong token"),
        )
        .await;
}

#[tokio::test]
async fn fund_token_past_deadline() {
    let mut interact = ContractInteract::new().await;
    let deadline = get_unix_timestamp() + 20;

    interact.upgrade(TARGET, deadline, TOKEN_ID_EGLD).await;

    wait_past_deadline(deadline);

    interact
        .fund_failed(
            TOKEN_ID_TTO,
            TOKEN_NONCE,
            TOKEN_AMOUNT,
            ExpectError(4, "cannot fund after deadline"),
        )
        .await;
}

#[tokio::test]
async fn test_query_balance() {
    let mut interact = ContractInteract::new().await;

    // Deploy
    let deadline = 1732516628u64;
    let token_identifier: &str = "TTO-281def";
    interact
        .deploy_succ(TARGET_CONTRACT, deadline, token_identifier)
        .await;

    // Check balance 0
    let initial_balance = interact.get_current_funds().await;
    println!("Initial balance: {:?}", initial_balance);
    assert_eq!(
        initial_balance,
        BigUint::zero(),
        "Initial balance should be 0"
    );

    // 2 funds
    let token_nonce = 0u64;
    let token_amount1 = 500000000000000000u128;
    let token_amount2 = 600000000000000000u128;
    interact
        .fund_succ(
            token_identifier,
            token_nonce,
            token_amount1,
            AddressType::Dan,
        )
        .await;

    interact
        .fund_succ(
            token_identifier,
            token_nonce,
            token_amount2,
            AddressType::Dan,
        )
        .await;

    // Check updated balance
    let final_balance = interact.get_current_funds().await;
    let expected_balance = token_amount1 + token_amount2;
    println!("Final balance: {:?}", final_balance);
    println!("Final balance should be {}", expected_balance);

    assert_eq!(
        final_balance,
        BigUint::from(expected_balance),
        "Balance amount should be {}",
        expected_balance
    );
}

#[tokio::test]
async fn test_query_deposit() {
    let mut interact = ContractInteract::new().await;

    let deposited_amount_before = interact.depositDan().await;

    // Bob funds
    let token_nonce = 0u64;
    let token_amount = 500000000000000000u128;
    interact
        .fund_succ(
            TOKEN_IDENTIFIER,
            token_nonce,
            token_amount,
            AddressType::Dan,
        )
        .await;

    //Sum Dan
    let deposited_amount = interact.depositDan().await;

    //dep amount = dep am before + token amount
    assert_eq!(
        deposited_amount,
        BigUint::from(token_amount).add(deposited_amount_before),
        "Deposited amount should be {:?}",
        token_amount
    );
}

// async fn test_chain_simulator_tx_send() {
//     let client = reqwest::Client::new();
//     let res = client
//         .post("localhost:8085/v1.0/transaction/simulate?checkSignature=false")
//         .body("
//             "sender": "erd13x29rvmp4qlgn4emgztd8jgvyzdj0p6vn37tqxas3v9mfhq4dy7shalqrx",
//             "receiver": "erd13x29rvmp4qlgn4emgztd8jgvyzdj0p6vn37tqxas3v9mfhq4dy7shalqrx",
//             "amount": "100000000000000000",
//             "nonce": 1,
//             "gasPrice": 1000000000,
//             "gasLimit": 50000,
//             "data": "",
//             "signature": ""
//           ")
//         .send()
//         .await;
//     println!("{:?}", res.unwrap());
// }
