// Code generated by the multiversx-sc proxy generator. DO NOT EDIT.

////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![allow(dead_code)]
#![allow(clippy::all)]

use multiversx_sc::proxy_imports::*;

pub struct CrowdfundingProxy;

impl<Env, From, To, Gas> TxProxyTrait<Env, From, To, Gas> for CrowdfundingProxy
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    type TxProxyMethods = CrowdfundingProxyMethods<Env, From, To, Gas>;

    fn proxy_methods(self, tx: Tx<Env, From, To, (), Gas, (), ()>) -> Self::TxProxyMethods {
        CrowdfundingProxyMethods { wrapped_tx: tx }
    }
}

pub struct CrowdfundingProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    wrapped_tx: Tx<Env, From, To, (), Gas, (), ()>,
}

#[rustfmt::skip]
impl<Env, From, Gas> CrowdfundingProxyMethods<Env, From, (), Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    Gas: TxGas<Env>,
{
    pub fn init<
        Arg0: ProxyArg<BigUint<Env::Api>>,
        Arg1: ProxyArg<u64>,
        Arg2: ProxyArg<EgldOrEsdtTokenIdentifier<Env::Api>>,
    >(
        self,
        target: Arg0,
        deadline: Arg1,
        token_identifier: Arg2,
    ) -> TxTypedDeploy<Env, From, NotPayable, Gas, ()> {
        self.wrapped_tx
            .payment(NotPayable)
            .raw_deploy()
            .argument(&target)
            .argument(&deadline)
            .argument(&token_identifier)
            .original_result()
    }
}

#[rustfmt::skip]
impl<Env, From, To, Gas> CrowdfundingProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    pub fn upgrade<
        Arg0: ProxyArg<BigUint<Env::Api>>,
        Arg1: ProxyArg<u64>,
        Arg2: ProxyArg<EgldOrEsdtTokenIdentifier<Env::Api>>,
    >(
        self,
        target: Arg0,
        deadline: Arg1,
        token_identifier: Arg2,
    ) -> TxTypedUpgrade<Env, From, To, NotPayable, Gas, ()> {
        self.wrapped_tx
            .payment(NotPayable)
            .raw_upgrade()
            .argument(&target)
            .argument(&deadline)
            .argument(&token_identifier)
            .original_result()
    }
}

#[rustfmt::skip]
impl<Env, From, To, Gas> CrowdfundingProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    pub fn fund(
        self,
    ) -> TxTypedCall<Env, From, To, (), Gas, ()> {
        self.wrapped_tx
            .raw_call("fund")
            .original_result()
    }

    pub fn status(
        self,
    ) -> TxTypedCall<Env, From, To, NotPayable, Gas, Status> {
        self.wrapped_tx
            .payment(NotPayable)
            .raw_call("status")
            .original_result()
    }

    pub fn get_current_funds(
        self,
    ) -> TxTypedCall<Env, From, To, NotPayable, Gas, BigUint<Env::Api>> {
        self.wrapped_tx
            .payment(NotPayable)
            .raw_call("getCurrentFunds")
            .original_result()
    }

    pub fn claim(
        self,
    ) -> TxTypedCall<Env, From, To, NotPayable, Gas, ()> {
        self.wrapped_tx
            .payment(NotPayable)
            .raw_call("claim")
            .original_result()
    }

    pub fn target(
        self,
    ) -> TxTypedCall<Env, From, To, NotPayable, Gas, BigUint<Env::Api>> {
        self.wrapped_tx
            .payment(NotPayable)
            .raw_call("getTarget")
            .original_result()
    }

    pub fn deadline(
        self,
    ) -> TxTypedCall<Env, From, To, NotPayable, Gas, u64> {
        self.wrapped_tx
            .payment(NotPayable)
            .raw_call("getDeadline")
            .original_result()
    }

    pub fn deposit<
        Arg0: ProxyArg<ManagedAddress<Env::Api>>,
    >(
        self,
        donor: Arg0,
    ) -> TxTypedCall<Env, From, To, NotPayable, Gas, BigUint<Env::Api>> {
        self.wrapped_tx
            .payment(NotPayable)
            .raw_call("getDeposit")
            .argument(&donor)
            .original_result()
    }

    pub fn cf_token_identifier(
        self,
    ) -> TxTypedCall<Env, From, To, NotPayable, Gas, EgldOrEsdtTokenIdentifier<Env::Api>> {
        self.wrapped_tx
            .payment(NotPayable)
            .raw_call("getCrowdfundingTokenIdentifier")
            .original_result()
    }
}

#[type_abi]
#[derive(TopEncode, TopDecode, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Status {
    FundingPeriod,
    Successful,
    Failed,
}
