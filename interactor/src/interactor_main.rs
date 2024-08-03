#![allow(non_snake_case)]

mod proxy;

use crowdfunding_esdt::endpoints::target;
use multiversx_sc_snippets::imports::*;
use multiversx_sc_snippets::sdk;
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

    async fn upgrade(&mut self, target: u128, deadline: u64, token_identifier: &str) {
        // let target = BigUint::<StaticApi>::from(target);
        // let token_identifier = EgldOrEsdtTokenIdentifier::esdt(token_identifier);

         self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .upgrade(BigUint::from(target), deadline, TokenIdentifier::from(token_identifier))
            .code(&self.contract_code)
            .code_metadata(CodeMetadata::UPGRADEABLE)
            .prepare_async()
            .run()
            .await;

        println!("upgrade completed");
    }

    async fn fund_egld(&mut self, token_amount: u128) {  ////////////
        // let token_id = String::new();
        // let token_nonce = 0u64;
        // let token_amount = BigUint::<StaticApi>::from(0u128);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address) 
            .to(self.state.current_address())
            .gas(NumExpr("30,000,000"))
            .typed(proxy::CrowdfundingProxy)
            .fund()
            .egld(
                BigUint::from(token_amount),
            )
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }


    async fn fund(&mut self, token_id: &str, token_nonce: u64, token_amount: u128) {  ////////////
        // let token_id = String::new();
        // let token_nonce = 0u64;
        // let token_amount = BigUint::<StaticApi>::from(0u128);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address) 
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

    async fn fundBob(&mut self, token_id: &str, token_nonce: u64, token_amount: u128) {  ////////////
        // let token_id = String::new();
        // let token_nonce = 0u64;
        // let token_amount = BigUint::<StaticApi>::from(0u128);

        let response = self
            .interactor
            .tx()
            .from(&self.user_address) 
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

        println!("FUNDBOB Result: {response:?}");
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

    async fn get_current_funds(&mut self) ->  BigUint<StaticApi>
    {

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

    async fn claim_fail(&mut self) {
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
    
   
      async fn depositBob(&mut self) -> BigUint<StaticApi>{
        let donor = "erd1spyavw0956vq68xj8y4tenjpq2wd5a9p2c6j8gsz7ztyrnpxrruqzu66jx";

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::CrowdfundingProxy)
            .deposit(&self.user_address)
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

}

#[tokio::test]
async fn test_deploy() {
    let mut interact = ContractInteract::new().await;
    let target = BigUint::<StaticApi>::from(5u128);
    let deadline = 1732516628u64;
    let token_identifier = "EGLD";
    interact.deploy(target, deadline, token_identifier).await;
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
    // let token_id = String::new();
    // let token_nonce = 0u64;
    // let token_amount = BigUint::<StaticApi>::from(0u128);
   
 //   let token_id1 = "EGLD";
    let token_id2 = "TTO-281def";
    let token_nonce = 0u64;
    let token_amount = 500000000000000000u128;
    // interact
    // .fund_egld(
    //     token_amount
    // )
    // .await;
    let target = 5u128;
    let deadline = 1732516628u64;


    interact.upgrade(target, deadline, token_id2).await;

    interact.fund(token_id2, token_nonce, token_amount).await;



}


#[tokio::test]
async fn test_queryBalance() {
    let mut interact = ContractInteract::new().await;

    // Fac deploy
    let target = BigUint::<StaticApi>::from(5u128);
    let deadline = 1732516628u64;
    let token_identifier: &str = "TTO-281def";
    interact.deploy(target, deadline, token_identifier).await;

    // Verific să am balanța inițială 0
    let initial_balance = interact.get_current_funds().await;
    println!("Initial balance: {:?}", initial_balance);
    assert_eq!(initial_balance, BigUint::zero(), "Initial balance should be 0");

    // 2 funds
    let token_nonce = 0u64;
    let token_amount1 = 500000000000000000u128;
    let token_amount2 = 600000000000000000u128;
    interact.fund(token_identifier, token_nonce, token_amount1).await;

    interact.fund(token_identifier, token_nonce, token_amount2).await;


   // Verific balanta după cele 2 funds
   let final_balance = interact.get_current_funds().await;
   let expected_balance = token_amount1 + token_amount2;
   println!("Final balance: {:?}", final_balance);
   println!( "Final balance should be {}", expected_balance);
  
   assert_eq!( final_balance, BigUint::from(expected_balance), "Balance amount should be {}", expected_balance);

}



#[tokio::test]
async fn test_QueryDeposit() {
    let mut interact = ContractInteract::new().await;

    //Deposit = Stocheaza suma donata de fiecare donator
    let deposited_amount_before = interact.depositBob().await;

    // User ul Bob face un fund
    let token_identifier: &str = "TTO-281def";
    let token_nonce = 0u64;
    let token_amount = 500000000000000000u128;  
    interact.fundBob(token_identifier, token_nonce, token_amount).await;

    //Suma de la bob
   
    let deposited_amount = interact.depositBob().await;
 
   println!( "Token amount {:?}", token_amount);

  println!("Deposited amount: {:?}", deposited_amount);

  //dep amount = dep am before + token amount
  assert_eq!(deposited_amount, BigUint::from(token_amount).add(deposited_amount_before), "Deposited amount should be {:?}", token_amount);
}

