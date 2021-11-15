# 🐮 prepper

> "Approve USDC when the base fee is < 10 gwei"

🐮 Prepper allows you to schedule transactions to be trigged in the future based on specific condtions.

Currently the only condition that prepper looks for is `base_fee_per_gas` but this will be extended in the future.

```json
{
	"trigger": {
		"metric": "base_fee_per_gas",
		"eq": "<",
		"value": 140000000000
	},
	"tx": {
		"from": "0x8626f6940e2eb28930efb4cef49b2d1f2c9c1199",
		"to": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
		"signed": "02f8b3827a690885174876e80085174876e80082b5d794c02aaa39b223fe8d0a0e5c4f27ead9083c756cc280b844095ea7b3000000000000000000000000dd2fd4581271e230360230f9337d5c0430bf44c0ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffc001a0be77cdf485193dff9cf7a4a0699401f5f405103aed79497b10afe159b622c67ba0725ad3f47897e7ec15a7ba7cc0939d7442aed34b4c72958187ceaa0e7c4bd8ec",
		"nonce": 8
	}
}
```

A user can add signed transaction to the service that get stored in the following structures

```rust
// our request above is deserialized into
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PreppedRequest {
    trigger: Trigger,
    tx: PreppedTxRequest,
}

// we then store it in
#[derive(Debug)]
struct RequestManager {
	requests: Vec<PreppedRequest>
}
```

Then as new blocks come in we iterate over the `RequestManager.requests` and check each trigger.

If the trigger is true then we fire off the request.

psuedo code:
```rust
// ~ snip ~

// example of whats happening
for req in locked_context.rmgmt.requests {
	//  we check if the base fee is below our target value
	if block_info.base_fee_per_gas < U256::from(trigger.value) {
	    println!("🪓 Executing");
	    // ~ snip ~
	    send_transaction(prepped_request);
	}
}
```

# Start local network

In a terminal fork ETH mainnet. This is our blockchain layer.

```bash
npx hardhat clean # if you want to start over
npx hardhat node
```

# Start app

In another terminal. This is essentially service layer.

```bash
cargo run
```

# Add signed transaction

In a last terminal we can drive our service. 

```bash
python3 allowance.py
# 0
```

Then send a request

```bash
cat reqs/basic.json | websocat -b ws://127.0.0.1:1337
```

Finally when it executes we should see 

```bash
python3 allowance.py
# 115792089237316195423570985008687907853269984665640564039457584007913129639935
```