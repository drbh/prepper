/**
 * @type import('hardhat/config').HardhatUserConfig
 */
module.exports = {
  solidity: "0.7.3",
  networks: {
    hardhat: {
      forking: {
        url: "http://localhost:8545",
        blockNumber: 13611653,
      },
      mining: {
        auto: false,
        interval: [3000, 6000],
      },
    },
  },
};