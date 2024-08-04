use multiversx_sc_scenario::imports::*;

use crowdfunding_esdt::*;
mod proxy;

const OWNER_ADDRESS: TestAddress = TestAddress::new("owner");
const USER_1_ADDRESS: TestAddress = TestAddress::new("user-1");
const USER_2_ADDRESS: TestAddress = TestAddress::new("user-2");
const SC_ADDRESS: TestSCAddress = TestSCAddress::new("crowdfunding-esdt");

const CODE_PATH: MxscPath = MxscPath::new("output/crowdfunding-esdt.mxsc.json");

const TARGET: u128 = 100;
const TOKEN_ID_TTO: TestTokenIdentifier = TestTokenIdentifier::new("TTO-281def");
const WRONG_TOKEN_ID: TestTokenIdentifier = TestTokenIdentifier::new("WRONG_TOKEN");

const TOKEN_NONCE: u64 = 0;
const DEADLINE: u64 = 1732516628;


fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new();

    blockchain.account(OWNER_ADDRESS);
    blockchain.account(USER_1_ADDRESS).esdt_balance(TOKEN_ID_TTO, 100).esdt_balance(WRONG_TOKEN_ID, 100);
    blockchain.account(USER_2_ADDRESS).esdt_balance(TOKEN_ID_TTO, 200).esdt_balance(WRONG_TOKEN_ID, 200);

    blockchain.register_contract(CODE_PATH, crowdfunding_esdt::ContractBuilder);
    blockchain
}

fn deploy(world: &mut ScenarioWorld){
    world
        .tx()
        .from(OWNER_ADDRESS)
        .typed(proxy::CrowdfundingProxy)
        .init(BigUint::from(TARGET), DEADLINE, TOKEN_ID_TTO)
        .code(CODE_PATH)
        .code_metadata(CodeMetadata::PAYABLE)
        .returns(ReturnsNewAddress)
        .new_address(SC_ADDRESS)
        .run();
}

fn fund(world: &mut ScenarioWorld, sender_address: TestAddress, token_amount: u64) {
    world
    .tx()
    .from(sender_address)
    .to(SC_ADDRESS)
    .gas(NumExpr("30,000,000"))
    .typed(proxy::CrowdfundingProxy)
    .fund()
    .egld_or_single_esdt(
        &EgldOrEsdtTokenIdentifier::esdt(TOKEN_ID_TTO),
        TOKEN_NONCE,
        &multiversx_sc::proxy_imports::BigUint::from(token_amount),
    )
    .run();
}

fn fund_wrong_token(world: &mut ScenarioWorld, sender_address: TestAddress, token_amount: u64) {
    world
    .tx()
    .from(sender_address)
    .to(SC_ADDRESS)
    .typed(proxy::CrowdfundingProxy)
    .fund()
    .egld_or_single_esdt(
        &EgldOrEsdtTokenIdentifier::esdt(WRONG_TOKEN_ID),
        TOKEN_NONCE,
        &multiversx_sc::proxy_imports::BigUint::from(token_amount)
    )
    .with_result(ExpectError(4, "wrong token"))
    .run();
}


fn check_deposit(world: &mut ScenarioWorld, sender_address: TestAddress, amount: u64) {
    world
    .query()
    .to(SC_ADDRESS)
    .typed(proxy::CrowdfundingProxy)
    .deposit(sender_address)
    .returns(ExpectValue(amount))
    .run()
}

fn check_sc_balance(world: &mut ScenarioWorld, address: TestSCAddress, amount: u64) {
    world
    .check_account(address)
    .esdt_balance(TOKEN_ID_TTO, amount);
}

fn check_user_balance(world: &mut ScenarioWorld, address: TestAddress, amount: u64) {
    world
    .check_account(address)
    .esdt_balance(TOKEN_ID_TTO, amount);
}

fn check_sc_status(world: &mut ScenarioWorld, expected_value: proxy::Status) {
   world
    .query()
    .to(SC_ADDRESS)
    .typed(proxy::CrowdfundingProxy)
    .status()
    .returns(ExpectValue(expected_value))
    .run();
}

fn set_sc_deadline(world: &mut ScenarioWorld, deadline: u64) {
    world
    .current_block()
    .block_timestamp(deadline);
}

fn claim(world: &mut ScenarioWorld, address: TestAddress) {
    world
    .tx()
    .from(address)
    .to(SC_ADDRESS)
    .typed(proxy::CrowdfundingProxy)
    .claim()
    .run();
}

fn claim_before_deadline(world: &mut ScenarioWorld, address: TestAddress) {
    world
    .tx()
    .from(address)
    .to(SC_ADDRESS)
    .typed(proxy::CrowdfundingProxy)
    .claim()
    .with_result(ExpectError(4, "cannot claim before deadline"))
    .run();
}

fn wrong_user_claim(world: &mut ScenarioWorld, address: TestAddress) {
    world
    .tx()
    .from(address)
    .to(SC_ADDRESS)
    .typed(proxy::CrowdfundingProxy)
    .claim()
    .with_result(ExpectError(4, "only owner can claim successful funding"))
    .run();
}



// Deploy test
#[test]
fn test_deploy() {
    let mut world = world();

    world.start_trace();
    
    deploy(&mut world);
    check_sc_status(&mut world, proxy::Status::FundingPeriod);
    
    world.write_scenario_trace("scenarios/trace_0.scen.json");
}

// Test if the amount is tranferred correctly 
#[test]
fn test_check_fund() {
    let mut world = world();
    world.start_trace();

    deploy(&mut world);
    fund(&mut world, USER_1_ADDRESS, 23);
    check_deposit(&mut world, USER_1_ADDRESS, 23);
    check_user_balance(&mut world, USER_1_ADDRESS, 77);

    
    world.write_scenario_trace("scenarios/trace_1.scen.json");
}

// Test if a different token was funded
#[test]
fn test_wrong_token_fund() {
    let mut world = world();
    world.start_trace();

    deploy(&mut world);
    fund_wrong_token(&mut world, USER_1_ADDRESS, 70);
    check_deposit(&mut world, USER_1_ADDRESS, 0);

    world.write_scenario_trace("scenarios/trace_2.scen.json")
}


#[test]
fn test_status_succes() {
    let mut world = world();
    world.start_trace();

    deploy(&mut world);

    // check FundingPeriod status before deadline
    check_sc_status(&mut world, proxy::Status::FundingPeriod);
    
    // fund to reach target (target = 100)
    fund(&mut world, USER_1_ADDRESS, 40);
    fund(&mut world, USER_2_ADDRESS, 60);
    
    check_deposit(&mut world, USER_1_ADDRESS, 40);
    check_deposit(&mut world, USER_2_ADDRESS, 60);


    check_sc_balance(&mut world, SC_ADDRESS, 100);  

    // set deadline
    set_sc_deadline(&mut world, DEADLINE);

    // check status successful after deadline
    check_sc_status(&mut world, proxy::Status::Successful);
        

    // check that only the owner can claim
    wrong_user_claim(&mut world, USER_1_ADDRESS);
    
    // check claim by owner
    claim(&mut world, OWNER_ADDRESS);
    check_user_balance(&mut world, OWNER_ADDRESS, 100);
    


    world.write_scenario_trace("scenarios/trace_3.scen.json")
}

#[test]
fn test_status_failed() {
    let mut world = world();
    world.start_trace();

    deploy(&mut world);

    // check FundingPeriod status before deadline
    check_sc_status(&mut world, proxy::Status::FundingPeriod);
    
    // fund doesn't reach target (current sum = 60)
    fund(&mut world, USER_1_ADDRESS, 40);
    fund(&mut world, USER_2_ADDRESS, 20);
    check_user_balance(&mut world, USER_1_ADDRESS, 60);
    check_user_balance(&mut world, USER_2_ADDRESS, 180);

    check_deposit(&mut world, USER_1_ADDRESS, 40);
    check_deposit(&mut world, USER_2_ADDRESS, 20);
    check_sc_balance(&mut world, SC_ADDRESS, 60);  

    // set deadline
    set_sc_deadline(&mut world, DEADLINE);

    // check status successful after deadline
    check_sc_status(&mut world, proxy::Status::Failed);
        
    
    // check that the owner cannot claim anything
    claim(&mut world, OWNER_ADDRESS);
    check_user_balance(&mut world, OWNER_ADDRESS, 0);


    // the users get their balance back
    claim(&mut world, USER_1_ADDRESS);
    claim(&mut world, USER_2_ADDRESS);

    // Check if balance was reverted to users
    check_user_balance(&mut world, USER_1_ADDRESS, 100);
    check_user_balance(&mut world, USER_2_ADDRESS, 200);

    world.write_scenario_trace("scenarios/trace_4.scen.json")
}


#[test]
fn test_claim_before_deadline() {
    let mut world = world();
    world.start_trace();

    deploy(&mut world);

    // fund to reach target (target = 100)
    fund(&mut world, USER_1_ADDRESS, 20);
    fund(&mut world, USER_2_ADDRESS, 30);
    
    check_deposit(&mut world, USER_1_ADDRESS, 20);
    check_deposit(&mut world, USER_2_ADDRESS, 30);

    claim_before_deadline(&mut world, OWNER_ADDRESS);

    // check FundingPeriod status before deadline
    check_sc_status(&mut world, proxy::Status::FundingPeriod);

    // check that owner did not claim anything
    check_user_balance(&mut world, OWNER_ADDRESS, 0);


    world.write_scenario_trace("scenarios/trace_5.scen.json")
}

