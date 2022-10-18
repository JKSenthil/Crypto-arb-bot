use ethers::types::{Address, U256};
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    RequestBuilder,
};

pub fn parse_address(addr: &str) -> Address {
    let addr = addr.strip_prefix("0x").unwrap_or(addr);
    addr.parse().unwrap()
}

pub fn convert_to_U256(input: u32, decimal: usize) -> U256 {
    // TODO use safe multiply?
    U256::from(input) * U256::exp10(decimal)
}

struct DebugTxClient {
    request_builder: RequestBuilder,
}

impl DebugTxClient {
    pub fn new(uri: &str) -> Self {
        let client = reqwest::Client::new();
        let request_builder = client
            .post(uri)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json");
        return Self {
            request_builder: request_builder,
        };
    }

    pub async fn debug_trace_transaction(
        &self,
        tx_hash: &str,
        encoded_function_preface: &str,
    ) -> Option<String> {
        let args = format!(
            "
        {{
            \"id\": 1,
            \"jsonrpc\": \"2.0\",
            \"method\": \"debug_traceTransaction\",
            \"params\": [
                \"{}\",
                {{
                    \"tracer\": \"callTracer\",
                    \"timeout\": \"5s\"
                }}
            ]
        }}
        ",
            tx_hash
        );
        let response = self
            .request_builder
            .try_clone()
            .unwrap()
            .body(args)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        match response.find(encoded_function_preface) {
            Some(index) => {
                let str = &response[index..index + 330];
                Some(str.to_string())
            }
            None => None,
        }
    }
}
