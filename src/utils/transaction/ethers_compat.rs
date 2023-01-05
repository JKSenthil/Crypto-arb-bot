//! ethers compatibility, this is mainly necessary so we can use all of `ethers` signers

use super::{
    EIP1559Transaction, EIP1559TransactionRequest, EIP2930TransactionRequest,
    EthTransactionRequest, LegacyTransaction, LegacyTransactionRequest, TransactionKind,
    TypedTransaction, TypedTransactionRequest,
};
use ethers::types::{
    transaction::{
        eip1559::Eip1559TransactionRequest as EthersEip1559TransactionRequest,
        eip2718::TypedTransaction as EthersTypedTransactionRequest,
        eip2930::Eip2930TransactionRequest as EthersEip2930TransactionRequest,
    },
    Address, NameOrAddress, Signature, Transaction as EthersTransaction,
    TransactionRequest as EthersLegacyTransactionRequest, TransactionRequest, H256, U256, U64,
};

impl From<TypedTransactionRequest> for EthersTypedTransactionRequest {
    fn from(tx: TypedTransactionRequest) -> Self {
        match tx {
            TypedTransactionRequest::Legacy(tx) => {
                let LegacyTransactionRequest {
                    nonce,
                    gas_price,
                    gas_limit,
                    kind,
                    value,
                    input,
                    chain_id,
                } = tx;
                EthersTypedTransactionRequest::Legacy(EthersLegacyTransactionRequest {
                    from: None,
                    to: kind.as_call().cloned().map(Into::into),
                    gas: Some(gas_limit),
                    gas_price: Some(gas_price),
                    value: Some(value),
                    data: Some(input),
                    nonce: Some(nonce),
                    chain_id: chain_id.map(Into::into),
                })
            }
            TypedTransactionRequest::EIP2930(tx) => {
                let EIP2930TransactionRequest {
                    chain_id,
                    nonce,
                    gas_price,
                    gas_limit,
                    kind,
                    value,
                    input,
                    access_list,
                } = tx;
                EthersTypedTransactionRequest::Eip2930(EthersEip2930TransactionRequest {
                    tx: EthersLegacyTransactionRequest {
                        from: None,
                        to: kind.as_call().cloned().map(Into::into),
                        gas: Some(gas_limit),
                        gas_price: Some(gas_price),
                        value: Some(value),
                        data: Some(input),
                        nonce: Some(nonce),
                        chain_id: Some(chain_id.into()),
                    },
                    access_list: access_list.into(),
                })
            }
            TypedTransactionRequest::EIP1559(tx) => {
                let EIP1559TransactionRequest {
                    chain_id,
                    nonce,
                    max_priority_fee_per_gas,
                    max_fee_per_gas,
                    gas_limit,
                    kind,
                    value,
                    input,
                    access_list,
                } = tx;
                EthersTypedTransactionRequest::Eip1559(EthersEip1559TransactionRequest {
                    from: None,
                    to: kind.as_call().cloned().map(Into::into),
                    gas: Some(gas_limit),
                    value: Some(value),
                    data: Some(input),
                    nonce: Some(nonce),
                    access_list: access_list.into(),
                    max_priority_fee_per_gas: Some(max_priority_fee_per_gas),
                    max_fee_per_gas: Some(max_fee_per_gas),
                    chain_id: Some(chain_id.into()),
                })
            }
        }
    }
}

// TODO fix this somehow?
impl From<EthersTransaction> for TypedTransaction {
    fn from(transaction: EthersTransaction) -> TypedTransaction {
        let kind: TransactionKind = match transaction.to {
            Some(to) => TransactionKind::Call(to),
            None => TransactionKind::Create,
        };

        if let Some(_) = transaction.max_fee_per_gas {
            let parity = if transaction.v == U64::one() {
                true
            } else {
                false
            };
            return TypedTransaction::EIP1559(EIP1559Transaction {
                chain_id: 137,
                nonce: transaction.nonce,
                max_priority_fee_per_gas: transaction.max_priority_fee_per_gas.unwrap(),
                max_fee_per_gas: transaction.max_fee_per_gas.unwrap(),
                gas_limit: transaction.gas,
                kind: kind,
                value: transaction.value,
                input: transaction.input,
                access_list: transaction.access_list.unwrap(),
                odd_y_parity: parity,
                r: {
                    let mut rarr = [0u8; 32];
                    transaction.r.to_big_endian(&mut rarr);
                    H256::from(rarr)
                },
                s: {
                    let mut rarr = [0u8; 32];
                    transaction.s.to_big_endian(&mut rarr);
                    H256::from(rarr)
                },
            });
        }
        TypedTransaction::Legacy(LegacyTransaction {
            nonce: transaction.nonce,
            gas_price: transaction.gas_price.unwrap(),
            gas_limit: transaction.gas,
            kind: kind,
            value: transaction.value,
            input: transaction.input,
            signature: Signature {
                r: transaction.r,
                s: transaction.s,
                v: transaction.v.as_u64(),
            },
        })
    }
}

impl From<TypedTransaction> for EthersTransaction {
    fn from(transaction: TypedTransaction) -> Self {
        let hash = transaction.hash();
        match transaction {
            TypedTransaction::Legacy(t) => EthersTransaction {
                hash,
                nonce: t.nonce,
                block_hash: None,
                block_number: None,
                transaction_index: None,
                from: Address::default(),
                to: None,
                value: t.value,
                gas_price: Some(t.gas_price),
                max_fee_per_gas: Some(t.gas_price),
                max_priority_fee_per_gas: Some(t.gas_price),
                gas: t.gas_limit,
                input: t.input.clone(),
                chain_id: t.chain_id().map(Into::into),
                v: t.signature.v.into(),
                r: t.signature.r,
                s: t.signature.s,
                access_list: None,
                transaction_type: Some(0u64.into()),
                other: Default::default(),
            },
            TypedTransaction::EIP2930(t) => EthersTransaction {
                hash,
                nonce: t.nonce,
                block_hash: None,
                block_number: None,
                transaction_index: None,
                from: Address::default(),
                to: None,
                value: t.value,
                gas_price: Some(t.gas_price),
                max_fee_per_gas: Some(t.gas_price),
                max_priority_fee_per_gas: Some(t.gas_price),
                gas: t.gas_limit,
                input: t.input.clone(),
                chain_id: Some(t.chain_id.into()),
                v: U64::from(t.odd_y_parity as u8),
                r: U256::from(t.r.as_bytes()),
                s: U256::from(t.s.as_bytes()),
                access_list: Some(t.access_list),
                transaction_type: Some(1u64.into()),
                other: Default::default(),
            },
            TypedTransaction::EIP1559(t) => EthersTransaction {
                hash,
                nonce: t.nonce,
                block_hash: None,
                block_number: None,
                transaction_index: None,
                from: Address::default(),
                to: None,
                value: t.value,
                gas_price: None,
                max_fee_per_gas: Some(t.max_fee_per_gas),
                max_priority_fee_per_gas: Some(t.max_priority_fee_per_gas),
                gas: t.gas_limit,
                input: t.input.clone(),
                chain_id: Some(t.chain_id.into()),
                v: U64::from(t.odd_y_parity as u8),
                r: U256::from(t.r.as_bytes()),
                s: U256::from(t.s.as_bytes()),
                access_list: Some(t.access_list),
                transaction_type: Some(2u64.into()),
                other: Default::default(),
            },
        }
    }
}

impl From<TransactionRequest> for EthTransactionRequest {
    fn from(req: TransactionRequest) -> Self {
        let TransactionRequest {
            from,
            to,
            gas,
            gas_price,
            value,
            data,
            nonce,
            ..
        } = req;
        EthTransactionRequest {
            from,
            to: to.and_then(|to| match to {
                NameOrAddress::Name(_) => None,
                NameOrAddress::Address(to) => Some(to),
            }),
            gas_price,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            gas,
            value,
            data,
            nonce,
            access_list: None,
            transaction_type: None,
        }
    }
}

impl From<EthTransactionRequest> for TransactionRequest {
    fn from(req: EthTransactionRequest) -> Self {
        let EthTransactionRequest {
            from,
            to,
            gas_price,
            gas,
            value,
            data,
            nonce,
            ..
        } = req;
        TransactionRequest {
            from,
            to: to.map(NameOrAddress::Address),
            gas,
            gas_price,
            value,
            data,
            nonce,
            chain_id: None,
        }
    }
}
