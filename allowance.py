from web3 import Web3

# WebsocketProvider:
w3 = Web3(Web3.WebsocketProvider('ws://127.0.0.1:8545'))

# allowance functions for erc20
abi = [
    {
        "constant": True,
        "inputs": [{"name": "", "type": "address"}, {"name": "", "type": "address"}],
        "name": "allowance",
        "outputs": [{"name": "", "type": "uint256"}],
        "payable": False,
        "stateMutability": "view", "type": "function"
    }
]

# weth
weth = w3.eth.contract(
    address=Web3.toChecksumAddress('0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2'), abi=abi)

# contract values we set in our tx
resp = weth.functions.allowance(
	Web3.toChecksumAddress("0x8626f6940e2eb28930efb4cef49b2d1f2c9c1199"),
	Web3.toChecksumAddress("0xdd2fd4581271e230360230f9337d5c0430bf44c0"),
).call()

print(resp)