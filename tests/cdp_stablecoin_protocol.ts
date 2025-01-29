import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { CdpStablecoinProtocol } from "../target/types/cdp_stablecoin_protocol";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { Keypair,SystemProgram, Commitment, PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { ASSOCIATED_TOKEN_PROGRAM_ID, Account, TOKEN_PROGRAM_ID, createMint, getAssociatedTokenAddressSync, getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";

const revenueShareToStabilityPool = 8000;
const protocolFee = 500;
const redemptionFee = 500;
const mintFee = 500;
const baseRate = 500;
const sigma = 200;
const stablecoinPriceFeed = "eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a"
const USDC_PYTH_ACCOUNT = new PublicKey("Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX");
const collateralAmount = new BN(0.1*LAMPORTS_PER_SOL);
const debtAmount1 = new BN(50000);
const debtAmount2 = new BN(20000);
const JITO_SOL_PRICE_FEED_ID = "67be9f519b95cf24338801051f9a808eff0a578ccb388db73b7f6fe1de019ffb";

const JITO_SOL_PYTH_ACCOUNT = new PublicKey("AxaxyeDT8JnWERSaTKvFXvPKkEdxnamKSqpWbsSjYg1g");


describe("cdp_stablecoin_protocol", () => {
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;
  anchor.setProvider(provider);
  const wallet = provider.wallet as NodeWallet;

  const program = anchor.workspace.CdpStablecoinProtocol as Program<CdpStablecoinProtocol>;

  let collateralMint1: PublicKey;
  let collateralAccount1_user1: Account;
  let collateralAccount1_user2: Account;
  let collateralVaultConfig1: PublicKey;
  let liquidationRewardsVault1: PublicKey;
  let collateralVault1: PublicKey;
  let collateralMint2: PublicKey;
  let collateralAccount2: Account;
  let collateralVaultConfig2: PublicKey;
  let liquidationRewardsVault2: PublicKey;
  let collateralVault2: PublicKey;
  let user1StableAta: PublicKey;
  let user2StableAta: PublicKey;
  let position1: PublicKey;
  let position2: PublicKey;
  let position2_user2: PublicKey;
  let stakeVault1: PublicKey;
  let stakeVault2: PublicKey;
  let stakeAccount1_user1: PublicKey;

  let wallet2 = Keypair.generate();


  const confirm = async (signature: string): Promise<string> => {
    const block = await connection.getLatestBlockhash();
    await connection.confirmTransaction({
      signature,
      ...block,
    });
    await log(signature);
    return signature;
  };
  const log = async (signature: string): Promise<string> => {
    console.log(
      `Your transaction signature: https://explorer.solana.com/transaction/${signature}?cluster=custom&customUrl=${connection.rpcEndpoint}`
    );
    return signature;
  };
  const protocolConfig = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("config"),
    ],
    program.programId
  )[0];
  
  const auth = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("auth"),
    ],
    program.programId
  )[0];

  const stableMint = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("stable"),
    ],
    program.programId
  )[0];
  // const stableMint = Keypair.generate()


  it("Create Collateral Mint and mint tokens", async() => {

    await anchor.getProvider().connection.requestAirdrop(wallet2.publicKey, 1 * anchor.web3.LAMPORTS_PER_SOL).then(confirm);

    collateralMint1 = await createMint(provider.connection, wallet.payer, wallet.publicKey, wallet.publicKey, 6);
    console.log("collateral Mint 1 : ", collateralMint1.toBase58());

    collateralMint2 = await createMint(provider.connection, wallet.payer, wallet.publicKey, wallet.publicKey, 6);
    console.log("collateral Mint 2 : ", collateralMint2.toBase58())

    collateralAccount1_user1 = await getOrCreateAssociatedTokenAccount(provider.connection, wallet.payer, collateralMint1, wallet.publicKey);

    collateralAccount1_user2 = await getOrCreateAssociatedTokenAccount(provider.connection, wallet.payer, collateralMint1, wallet2.publicKey);

    const tx1 = await mintTo(provider.connection, wallet.payer, collateralMint1, collateralAccount1_user1.address, wallet.payer, 1000000000);

    const tx2 = await mintTo(provider.connection, wallet.payer, collateralMint1, collateralAccount1_user2.address, wallet.payer, 1000000000);

    collateralAccount2 = await getOrCreateAssociatedTokenAccount(provider.connection, wallet.payer, collateralMint2, wallet.publicKey);

    const tx3 = await mintTo(provider.connection, wallet.payer, collateralMint2, collateralAccount2.address, wallet.payer, 1000000000);

    user1StableAta = getAssociatedTokenAddressSync(stableMint, wallet.publicKey)

    user2StableAta = getAssociatedTokenAddressSync(stableMint, wallet2.publicKey)

    collateralVaultConfig1 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("collateral"),
        collateralMint1.toBuffer()
      ],
      program.programId
    )[0];
    collateralVault1 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("collateral_vault"),
        collateralMint1.toBuffer()
      ],
      program.programId
    )[0];

    liquidationRewardsVault1 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("liquidation_rewards_vault"),
        collateralMint1.toBuffer()
      ],
      program.programId
    )[0];

    collateralVaultConfig2 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("collateral"),
        collateralMint2.toBuffer()
      ],
      program.programId
    )[0];
    collateralVault2 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("collateral_vault"),
        collateralMint2.toBuffer()
      ],
      program.programId
    )[0];

    liquidationRewardsVault2 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("liquidation_rewards_vault"),
        collateralMint2.toBuffer()
      ],
      program.programId
    )[0];

    stakeVault1 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("stake_vault"),
        stableMint.toBuffer(),
        collateralMint1.toBuffer()
      ],
      program.programId
    )[0];

    stakeVault2 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("stake_vault"),
        stableMint.toBuffer(),
        collateralMint2.toBuffer()
      ],
      program.programId
    )[0];

    stakeAccount1_user1 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("stake"),
        wallet.publicKey.toBuffer(),
        collateralMint1.toBuffer()
      ],
      program.programId
    )[0];


  });

  it("Initialize Protocol Config", async () => {

    // Add your test here.
    const tx = await program.methods.initializeProtocolConfig(
      protocolFee,
      redemptionFee,
      mintFee,
      baseRate,
      sigma,
      stablecoinPriceFeed,
      revenueShareToStabilityPool
    )
    .accountsPartial({
      admin: wallet.publicKey,
      protocolConfig,
      stableMint: stableMint,
      auth,
    })
    .signers([wallet.payer])
    .rpc()
    .then(confirm);
    console.log("Your transaction signature", tx);
  });

  it("Initialize Collateral Config 1", async () => {

    // Add your test here.
    const tx = await program.methods.initializeCollateralVault(
      JITO_SOL_PRICE_FEED_ID
    )
    .accountsPartial({
      admin: wallet.publicKey,
      collateralMint: collateralMint1,
      collateralVaultConfig: collateralVaultConfig1,
      protocolConfig,
      auth,
      collateralVault: collateralVault1,
      liquidationRewardsVault: liquidationRewardsVault1,
      stakeVault: stakeVault1,
      stableMint,
    })
    .signers([wallet.payer])
    .rpc()
    .then(confirm);
    console.log("Your transaction signature", tx);
  });

  it("Initialize Collateral Config 2", async () => {

    // Add your test here.
    const tx = await program.methods.initializeCollateralVault(
      JITO_SOL_PRICE_FEED_ID
    )
    .accountsPartial({
      admin: wallet.publicKey,
      collateralMint: collateralMint2,
      collateralVaultConfig: collateralVaultConfig2,
      protocolConfig,
      auth,
      collateralVault: collateralVault2,
      liquidationRewardsVault: liquidationRewardsVault2,
      stakeVault: stakeVault2,
      stableMint,
    })
    .signers([wallet.payer])
    .rpc()
    .then(confirm);
    console.log("Your transaction signature", tx);
  });

  it("Open debt position 1 with collateral mint 1 by user 1", async () => {

    position1 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("position"),
        wallet.publicKey.toBuffer(),
        collateralMint1.toBuffer()
      ],
      program.programId
    )[0];
    
    const tx = await program.methods.openPosition(
      collateralAmount,
      debtAmount1
    )
    .accountsPartial({
      user: wallet.publicKey,
      collateralMint: collateralMint1,
      stableMint: stableMint,
      protocolConfig,
      auth,
      userAta: collateralAccount1_user1.address,
      userStableAta: user1StableAta,
      collateralVaultConfig: collateralVaultConfig1,
      position: position1,
      priceFeed: JITO_SOL_PYTH_ACCOUNT,
      collateralVault: collateralVault1,
    })
    .signers([wallet.payer, ])
    .rpc({skipPreflight:true})
    .then(confirm);
    console.log("Your transaction signature", tx);
  });

  it("Open debt position 2 with collateral mint 1 by user 2", async () => {

    position2_user2 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("position"),
        wallet2.publicKey.toBuffer(),
        collateralMint1.toBuffer()
      ],
      program.programId
    )[0];
    
    const tx = await program.methods.openPosition(
      collateralAmount,
      debtAmount2
    )
    .accountsPartial({
      user: wallet2.publicKey,
      collateralMint: collateralMint1,
      stableMint: stableMint,
      protocolConfig,
      auth,
      userAta: collateralAccount1_user2.address,
      userStableAta: user2StableAta,
      collateralVaultConfig: collateralVaultConfig1,
      position: position2_user2,
      priceFeed: JITO_SOL_PYTH_ACCOUNT,
      collateralVault: collateralVault1,
    })
    .signers([wallet2, ])
    .rpc({skipPreflight:true})
    .then(confirm);
    console.log("Your transaction signature", tx);
  });

  it("Open debt position with collateral mint 2", async () => {

    position2 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("position"),
        wallet.publicKey.toBuffer(),
        collateralMint2.toBuffer()
      ],
      program.programId
    )[0];
    
    const tx = await program.methods.openPosition(
      collateralAmount,
      debtAmount2
    )
    .accountsPartial({
      user: wallet.publicKey,
      collateralMint: collateralMint2,
      stableMint: stableMint,
      protocolConfig,
      auth,
      userAta: collateralAccount2.address,
      userStableAta: user1StableAta,
      collateralVaultConfig: collateralVaultConfig2,
      position: position2,
      priceFeed: JITO_SOL_PYTH_ACCOUNT,
      collateralVault: collateralVault2,
    })
    .signers([wallet.payer, ])
    .rpc({skipPreflight:true})
    .then(confirm);
    console.log("Your transaction signature", tx);
  });


  it("Close debt position", async () => {

    position1 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("position"),
        wallet.publicKey.toBuffer(),
        collateralMint1.toBuffer()
      ],
      program.programId
    )[0];
    
    const tx = await program.methods.closePosition()
    .accountsPartial({
      user: wallet.publicKey,
      collateralMint: collateralMint1,
      stableMint: stableMint,
      protocolConfig,
      auth,
      userAta: collateralAccount1_user1.address,
      userStableAta: user1StableAta,
      collateralVaultConfig: collateralVaultConfig1,
      position: position1,
      priceFeed: JITO_SOL_PYTH_ACCOUNT,
      collateralVault: collateralVault1,
    })
    .signers([wallet.payer, ])
    .rpc({skipPreflight:true})
    .then(confirm);
    console.log("Your transaction signature", tx);
  });


  it("Stake Stability Tokens", async () => {

    const stakeAmount = new BN(1000);
    
    const userStableBalance = await connection.getTokenAccountBalance(user1StableAta);
    console.log("User's stable token balance before staking:", {
      rawAmount: userStableBalance.value.amount,
      uiAmount: userStableBalance.value.uiAmount,
      decimals: userStableBalance.value.decimals
    });
    
    const tx = await program.methods.stakeStableTokens(
      stakeAmount
    )
    .accountsPartial({
      user: wallet.publicKey,
      stakeAccount: stakeAccount1_user1,
      stableMint: stableMint,
      userStableAta: user1StableAta,
      auth,
      stakeVault: stakeVault1,
      collateralVaultConfig: collateralVaultConfig1,
      protocolConfig,
    })
    .signers([wallet.payer, ])
    .rpc({skipPreflight:true})
    .then(confirm);
    console.log("Your transaction signature", tx);
  });

  it("UnStake Stability Tokens", async () => {
    
    const tx = await program.methods.unstakeStableTokens()
    .accountsPartial({
      user: wallet.publicKey,
      stakeAccount: stakeAccount1_user1,
      stableMint: stableMint,
      userStableAta: user1StableAta,
      auth,
      stakeVault: stakeVault1,
      collateralVaultConfig: collateralVaultConfig1,
      protocolConfig,
    })
    .signers([wallet.payer, ])
    .rpc({skipPreflight:true})
    .then(confirm);
    console.log("Your transaction signature", tx);

    const userStableBalance = await connection.getTokenAccountBalance(user1StableAta);
    console.log("User's stable token balance after unstaking:", {
      rawAmount: userStableBalance.value.amount,
      uiAmount: userStableBalance.value.uiAmount,
      decimals: userStableBalance.value.decimals
    });
  });

  xit("liquidate position", async () => {

    position2_user2 = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("position"),
        wallet2.publicKey.toBuffer(),
        collateralMint1.toBuffer()
      ],
      program.programId
    )[0];
    
    const tx = await program.methods.liquidatePosition()
    .accountsPartial({
      liquidator: wallet.publicKey,
      user: wallet2.publicKey,
      collateralMint: collateralMint1,
      stableMint: stableMint,
      protocolConfig,
      auth,
      userAta: collateralAccount1_user2.address,
      userStableAta: user2StableAta,
      collateralVaultConfig: collateralVaultConfig1,
      position: position2_user2,
      priceFeed: JITO_SOL_PYTH_ACCOUNT,
      collateralVault: collateralVault1,
      liquidationRewardsVault: liquidationRewardsVault1,
      stakeVault: stakeVault1,
    })
    .signers([wallet.payer, ])
    .rpc({skipPreflight:true})
    .then(confirm);
    console.log("Your transaction signature", tx);
  });

  it("update interest", async () => {
    // Wait 5 seconds before updating interest rate to get the higher elapsed time section of the code
    await new Promise(resolve => setTimeout(resolve, 5000));

    const tx = await program.methods.updateInterestRate()
    .accountsPartial({
      user: wallet.publicKey,
      protocolConfig,
      stablecoinPriceFeed: USDC_PYTH_ACCOUNT,
    })
    .signers([wallet.payer, ])
    .rpc({skipPreflight:true})
    .then(confirm);
    console.log("Your transaction signature", tx);
  });


  xit("withdraw liquidation reward", async () => {
    
    const tx = await program.methods.claimStakeReward()
    .accountsPartial({
      user: wallet.publicKey,
      collateralMint: collateralMint1,
      userAta: collateralAccount1_user1.address,
      protocolConfig,
      auth,
      collateralVaultConfig: collateralVaultConfig1,
      liquidationRewardsVault: liquidationRewardsVault1,
      stakeAccount: stakeAccount1_user1,
    })
    .signers([wallet.payer, ])
    .rpc({skipPreflight:true})
    .then(confirm);
    console.log("Your transaction signature", tx);
  });

  it("Verify upfront cost mechanism", async () => {
    // Get initial balances
    const initialUserStableBalance = await connection.getTokenAccountBalance(user1StableAta);
    const initialStakeVaultBalance = await connection.getTokenAccountBalance(stakeVault1);
    
    console.log("Initial balances:", {
      userStable: initialUserStableBalance.value,
      stakeVault: initialStakeVaultBalance.value
    });

    // Open position
    const testCollateralAmount = new BN(0.1 * LAMPORTS_PER_SOL);
    const testDebtAmount = new BN(50000);
    
    const testPosition = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("position"),
        wallet.publicKey.toBuffer(),
        collateralMint1.toBuffer()
      ],
      program.programId
    )[0];
    
    const openTx = await program.methods.openPosition(
      testCollateralAmount,
      testDebtAmount
    )
    .accountsPartial({
      user: wallet.publicKey,
      collateralMint: collateralMint1,
      stableMint: stableMint,
      protocolConfig,
      auth,
      userAta: collateralAccount1_user1.address,
      userStableAta: user1StableAta,
      collateralVaultConfig: collateralVaultConfig1,
      position: testPosition,
      priceFeed: JITO_SOL_PYTH_ACCOUNT,
      collateralVault: collateralVault1,
      stakeVault: stakeVault1,
    })
    .signers([wallet.payer])
    .rpc({skipPreflight:true})
    .then(confirm);
    
    // Get balances after opening position
    const afterOpenUserBalance = await connection.getTokenAccountBalance(user1StableAta);
    const afterOpenStakeVaultBalance = await connection.getTokenAccountBalance(stakeVault1);
    
    console.log("After opening position:", {
      userStable: afterOpenUserBalance.value,
      stakeVault: afterOpenStakeVaultBalance.value,
      mintedAmount: testDebtAmount.toString(),
      upfrontCost: afterOpenStakeVaultBalance.value.amount
    });

    // Close position
    const closeTx = await program.methods.closePosition()
    .accountsPartial({
      user: wallet.publicKey,
      collateralMint: collateralMint1,
      stableMint: stableMint,
      protocolConfig,
      auth,
      userAta: collateralAccount1_user1.address,
      userStableAta: user1StableAta,
      collateralVaultConfig: collateralVaultConfig1,
      position: testPosition,
      priceFeed: JITO_SOL_PYTH_ACCOUNT,
      collateralVault: collateralVault1,
    })
    .signers([wallet.payer])
    .rpc({skipPreflight:true})
    .then(confirm);

    // Get final balances
    const finalUserBalance = await connection.getTokenAccountBalance(user1StableAta);
    const finalStakeVaultBalance = await connection.getTokenAccountBalance(stakeVault1);
    
    console.log("Final balances:", {
      userStable: finalUserBalance.value,
      stakeVault: finalStakeVaultBalance.value
    });

    // Calculate and display differences
    const userBalanceDiff = Number(afterOpenUserBalance.value.amount) - Number(initialUserStableBalance.value.amount);
    const stakeVaultDiff = Number(afterOpenStakeVaultBalance.value.amount) - Number(initialStakeVaultBalance.value.amount);
    
    console.log("Balance changes:", {
      userStableChange: userBalanceDiff,
      stakeVaultChange: stakeVaultDiff,
      upfrontCostPercentage: (stakeVaultDiff / Number(testDebtAmount) * 100).toFixed(2) + "%"
    });
  });

});
