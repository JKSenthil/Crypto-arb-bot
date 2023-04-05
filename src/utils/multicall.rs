use std::sync::Arc;

use ethers::{
    abi::{Detokenize, Function, Token},
    prelude::{abigen, builders::ContractCall},
    providers::Middleware,
    types::{Address, Bytes, NameOrAddress, U256},
};

abigen!(MulticallContract, "abis/Multicall.json");

#[derive(Clone, Debug)]
pub struct Call {
    target: Address,
    data: Bytes,
    value: U256,
    function: Function,
}

// https://github.com/mds1/multicall
// need to write custom multicall because
// library can't handle errors
pub struct Multicall<M> {
    calls: Vec<Call>,
    contract: MulticallContract<M>,
}

impl<M: Middleware> Multicall<M> {
    pub fn new(client: Arc<M>) -> Self {
        let address = "0xcA11bde05977b3631167028862bE2a173976CA11"
            .parse::<Address>()
            .unwrap();
        let contract = MulticallContract::new(address, client);
        Self {
            calls: vec![],
            contract,
        }
    }

    pub fn add_call<D: Detokenize>(&mut self, call: ContractCall<M, D>) {
        match (call.tx.to(), call.tx.data()) {
            (Some(NameOrAddress::Address(target)), Some(data)) => {
                let call = Call {
                    target: *target,
                    data: data.clone(),
                    value: call.tx.value().cloned().unwrap_or_default(),
                    function: call.function,
                };
                self.calls.push(call);
            }
            _ => {}
        }
    }

    fn as_aggregate_3(&self) -> ContractCall<M, Vec<Result>> {
        // Map the calls vector into appropriate types for `aggregate_3` function
        let calls: Vec<Call3> = self
            .calls
            .iter()
            .map(|call| Call3 {
                target: call.target,
                call_data: call.data.clone(),
                allow_failure: true,
            })
            .collect();

        // Construct the ContractCall for `aggregate_3` function to broadcast the transaction
        let contract_call = self.contract.aggregate_3(calls);
        contract_call
    }

    pub async fn call_raw(&self) -> Vec<Option<Vec<Token>>> {
        let call: ContractCall<M, Vec<Result>> = self.as_aggregate_3();
        let return_data: Vec<Result> = call.call().await.unwrap();

        let output = self
            .calls
            .iter()
            .zip(&return_data)
            .map(|(call, res)| {
                if res.success {
                    // Decode using call.function
                    let res_tokens = call.function.decode_output(&res.return_data);
                    match res_tokens {
                        Ok(tokens) => {
                            return Some(tokens);
                        }
                        Err(_) => return None,
                    };
                }
                None
            })
            .collect();

        return output;
    }
}
