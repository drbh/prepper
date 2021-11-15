use std::env;
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers_core::types::Eip1559TransactionRequest;
use message_io::network::{NetEvent, Transport};
use message_io::node::{self};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;

// representation of a signed transaction
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PreppedTxRequest {
    to: String,
    from: String,
    signed: String,
    nonce: u64,
}

// the values that trigger this transaction
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Trigger {
    metric: String,
    eq: String,
    value: u64,
}

// trigger and signed tx packaged
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PreppedRequest {
    trigger: Trigger,
    tx: PreppedTxRequest,
}

// struct to hold pending transactions
#[derive(Debug, Clone)]
struct RequestManager {
    requests: Vec<PreppedRequest>,
}

// the shared memory that will be used across threads
#[derive(Debug, Clone)]
struct Context {
    provider: Provider<Ws>,
    wallet: LocalWallet,
    rmgmt: RequestManager,
}

// Context defaults to using a specific websocket node and an interval time
impl Context {
    async fn new() -> Self {
        let url = env::var("RPC_PROVIDER").unwrap_or("ws://localhost:8545".to_string());
        let ws: Ws = Ws::connect(url).await.unwrap();

        // init a wallet
        let mut wallet: LocalWallet =
            "df57089febbacf7ba0bc227dafbffa9fc08a93fdc68e1e42411a14efcf23656e"
                .parse() // this key is from hardhat Account #19
                .unwrap();
        wallet = wallet.with_chain_id(31337u64);
        // wallet = wallet.with_chain_id(1u64);

        Self {
            provider: Provider::new(ws).interval(Duration::from_millis(1000)),
            wallet,
            rmgmt: RequestManager { requests: vec![] },
        }
    }
}

// send a presigned transaction
async fn send_transaction(_transaction: PreppedRequest, context: Arc<Mutex<Context>>) -> anyhow::Result<()> {
    let context = context.lock().unwrap();
    // let address = context.wallet.address();

    // extract bytes and convert to send
    let bys = hex::decode(_transaction.tx.signed).unwrap();
    let tx_bytes = Bytes::from(bys);

    // send the transaction
    let pending = context.provider.send_raw_transaction(tx_bytes).await?;
    println!("{:?}", pending);

    Ok(())
}

// craft a transaction and sign it
#[allow(dead_code)]
async fn craft_transaction(nonce: u64, context: Arc<Mutex<Context>>) -> anyhow::Result<()> {
    let context = context.lock().unwrap();
    let address = context.wallet.address();
    println!("My Address:\t\t{:?}", address);

    // manual construct an Approve call for ERC20
    let method_id = "095ea7b3";
    let spender = "000000000000000000000000dd2fd4581271e230360230f9337d5c0430bf44c0";
    let value = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
    let hex_input = format!("{}{}{}", method_id, spender, value);

    let tx = Eip1559TransactionRequest::new()
        .from(address)
        .to("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".parse::<Address>()?)
        .nonce(nonce)
        .gas(46_551) // naive gas numbers
        .max_fee_per_gas(100_000_000_000u64)
        .max_priority_fee_per_gas(100_000_000_000u64)
        .data(hex::decode(hex_input).unwrap());

    let tx = TypedTransaction::Eip1559(tx);
    let signature = context.wallet.sign_transaction(&tx).await?;
    let _tx_bytes = tx.rlp_signed(context.wallet.chain_id(), &signature);

    let tx_as_hex: String = hex::encode(_tx_bytes);

    println!("{:?}", tx_as_hex);


    Ok(())
}

async fn stream_completed_blocks(context: Arc<Mutex<Context>>) -> anyhow::Result<()> {
    // lock memory to access our provider then we clone it into this scope and drop out lock
    // lock dropped so we can open and close in loop
    let locked_internal_memory = context.lock().unwrap();
    
    // show output
    println!("━━ My Address:\t\t{:?}", locked_internal_memory.wallet.address());

    let cloned_locked_internal_memory = locked_internal_memory.provider.clone();
    let mut stream = cloned_locked_internal_memory.watch_blocks().await?;
    drop(locked_internal_memory);

    // iterate over blocks as they come in
    while let Some(block) = stream.next().await {
        let mut locked_context = context.lock().unwrap();

        let block_info = locked_context.provider.get_block(block).await?.unwrap();
        let _current_gas_price = locked_context.provider.get_gas_price().await?;
        println!(".");
        println!("├─ Number:\t\t{:?}", block_info.number.unwrap());
        println!("╰─ Current Gas:\t\t{:#?}", _current_gas_price);

        locked_context.rmgmt.requests.retain(|r| {
            let trigger = &r.trigger;
            //
            println!(
                "━━ Checking:\t{} {}",
                block_info.base_fee_per_gas.unwrap(),
                trigger.value
            );

            if block_info.base_fee_per_gas.unwrap() < U256::from(trigger.value) {
                println!("🪓 Executing");
                println!("{:#?}", r);

                // copy request to send into thread
                let prepped_request = r.clone();

                // clone context to send into thread
                let cloned_for_execution_thread = context.clone();

                // start new thread send a tx and wait for completion
                thread::spawn(move || {
                    let rt = Runtime::new().unwrap();
                    let _result = rt.block_on(async {
                        send_transaction(prepped_request, cloned_for_execution_thread).await.unwrap();
                    });
                });

                return false;
            };
            return true;
        })
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // shared memory variables
    let memory_db: Arc<Mutex<Context>> = Arc::new(Mutex::new(Context::new().await));
    let memory_db_for_stream = memory_db.clone();
    let memory_db_for_socket = memory_db.clone();

    println!("🧱 Streaming Blocks");
    thread::spawn(move || {
        // run async function until it ends
        Runtime::new().unwrap().block_on(async {
            stream_completed_blocks(memory_db_for_stream).await.unwrap();
        });
    });


    craft_transaction(8, memory_db_for_socket.clone()).await?;

    // start a server for WS
    let (handler, listener) = node::split::<()>();
    let (_server, _) = handler
        .network()
        .listen(Transport::Ws, "0.0.0.0:1337")
        .unwrap();

    // start listening
    println!("🚗 Running Server");
    listener.for_each(move |event| match event.network() {
        NetEvent::Connected(_, _) => unreachable!(), // Used for explicit connections.
        NetEvent::Accepted(_endpoint, _listener) => {
            println!("━━ {}", "Accepted");
        }
        NetEvent::Message(_sender, data) => {
            if let Ok(message_wrapper) =
                serde_json::from_str::<PreppedRequest>(&String::from_utf8_lossy(&data))
            {
                // incoming message when parsed
                println!("{:#?}", message_wrapper);
                let cloned_database = memory_db_for_socket.clone();
                let _nonce = message_wrapper.tx.nonce;
                let mut locked_result = cloned_database.lock().unwrap();
                locked_result.rmgmt.requests.push(message_wrapper);
                drop(locked_result);
            }
        }
        NetEvent::Disconnected(_endpoint) => {
            println!("━━ {}", "Dropped");
        }
    });

    Ok(())
}
