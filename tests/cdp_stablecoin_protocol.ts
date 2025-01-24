import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { CdpStablecoinProtocol } from "../target/types/cdp_stablecoin_protocol";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { Keypair,SystemProgram, Commitment, PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { ASSOCIATED_TOKEN_PROGRAM_ID, Account, TOKEN_PROGRAM_ID, createMint, getAssociatedTokenAddressSync, getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";
const protocolFee = 500;
const redemptionFee = 500;
const mintFee = 500;
const baseRate = 100;
const sigma = 20;
const stablecoinPriceFeed = "eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a"
const collateralAmount = new BN(0.1*LAMPORTS_PER_SOL);
const debtAmount = new BN(10);
const JITO_SOL_PRICE_FEED_ID = "67be9f519b95cf24338801051f9a808eff0a578ccb388db73b7f6fe1de019ffb";

const JITO_SOL_PYTH_ACCOUNT = new PublicKey("AxaxyeDT8JnWERSaTKvFXvPKkEdxnamKSqpWbsSjYg1g");


describe("cdp_stablecoin_protocol", () => {
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;
  anchor.setProvider(provider);
  const wallet = provider.wallet as NodeWallet;

  const program = anchor.workspace.CdpStablecoinProtocol as Program<CdpStablecoinProtocol>;

  let collateralMint: PublicKey;
  let collateralAccount: Account;
  let collateralVaultConfig: PublicKey;
  let collateralVault: PublicKey;
  let userStableAta: PublicKey;
  let position: PublicKey;


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
    collateralMint = await createMint(provider.connection, wallet.payer, wallet.publicKey, wallet.publicKey, 6);
    console.log("COMP Mint: ", collateralMint.toBase58());

    collateralAccount = await getOrCreateAssociatedTokenAccount(provider.connection, wallet.payer, collateralMint, wallet.publicKey);

    const tx = await mintTo(provider.connection, wallet.payer, collateralMint, collateralAccount.address, wallet.payer, 1000000000);

    userStableAta = getAssociatedTokenAddressSync(stableMint, wallet.publicKey)

    collateralVaultConfig = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("collateral"),
        collateralMint.toBuffer()
      ],
      program.programId
    )[0];
    collateralVault = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("collateral_vault"),
        collateralMint.toBuffer()
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
      stablecoinPriceFeed
    )
    .accountsPartial({
      admin: wallet.publicKey,
      protocolConfig,
      auth,
      stableMint: stableMint,
    })
    .signers([wallet.payer])
    .rpc()
    .then(confirm);
    console.log("Your transaction signature", tx);
  });

  it("Initialize Collateral Config", async () => {

    // Add your test here.
    const tx = await program.methods.initializeCollateralVault(
      JITO_SOL_PRICE_FEED_ID
    )
    .accountsPartial({
      admin: wallet.publicKey,
      collateralMint,
      collateralVaultConfig,
      protocolConfig,
      auth,
      collateralVault,
    })
    .signers([wallet.payer])
    .rpc()
    .then(confirm);
    console.log("Your transaction signature", tx);
  });

  it("Open debt position", async () => {

    position = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("position"),
        wallet.publicKey.toBuffer(),
        collateralMint.toBuffer()
      ],
      program.programId
    )[0];
    
    const tx = await program.methods.openPosition(
      collateralAmount,
      debtAmount
    )
    .accountsPartial({
      user: wallet.publicKey,
      collateralMint,
      stableMint: stableMint,
      protocolConfig,
      auth,
      userAta: collateralAccount.address,
      userStableAta,
      collateralVaultConfig,
      position,
      priceFeed: JITO_SOL_PYTH_ACCOUNT,
      collateralVault,
    })
    .signers([wallet.payer, ])
    .rpc({skipPreflight:true})
    .then(confirm);
    console.log("Your transaction signature", tx);
  });


  it("Close debt position", async () => {

    position = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("position"),
        wallet.publicKey.toBuffer(),
        collateralMint.toBuffer()
      ],
      program.programId
    )[0];
    
    const tx = await program.methods.closePosition()
    .accountsPartial({
      user: wallet.publicKey,
      collateralMint,
      stableMint: stableMint,
      protocolConfig,
      auth,
      userAta: collateralAccount.address,
      userStableAta,
      collateralVaultConfig,
      position,
      priceFeed: JITO_SOL_PYTH_ACCOUNT,
      collateralVault,
    })
    .signers([wallet.payer, ])
    .rpc({skipPreflight:true})
    .then(confirm);
    console.log("Your transaction signature", tx);
  });

});
