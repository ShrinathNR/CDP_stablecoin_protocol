import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { CdpStablecoinProtocol } from "../target/types/cdp_stablecoin_protocol";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { Keypair,SystemProgram, Commitment, PublicKey } from "@solana/web3.js";
import { ASSOCIATED_TOKEN_PROGRAM_ID, Account, TOKEN_PROGRAM_ID, createMint, getAssociatedTokenAddressSync, getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";
const protocolFee = 500;
const redemptionFee = 500;
const mintFee = 500;
const baseRate = 100;
const sigma = 20;
const stablecoinPriceFeed = "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d"
const collateralPriceFeed = "67be9f519b95cf24338801051f9a808eff0a578ccb388db73b7f6fe1de019ffb"



describe("cdp_stablecoin_protocol", () => {
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;
  anchor.setProvider(provider);
  const wallet = provider.wallet as NodeWallet;

  const program = anchor.workspace.CdpStablecoinProtocol as Program<CdpStablecoinProtocol>;

  let collateralMint: anchor.web3.PublicKey;
  let collateralAccount: Account;


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

  it("Create Collateral Mint and mint tokens", async() => {
    collateralMint = await createMint(provider.connection, wallet.payer, wallet.publicKey, wallet.publicKey, 6);
    console.log("COMP Mint: ", collateralMint.toBase58());

    collateralAccount = await getOrCreateAssociatedTokenAccount(provider.connection, wallet.payer, collateralMint, wallet.publicKey);

    const tx = await mintTo(provider.connection, wallet.payer, collateralMint, collateralAccount.address, wallet.payer, 1000000000);

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
      stableMint,

    })
    .signers([wallet.payer])
    .rpc()
    .then(confirm);
    console.log("Your transaction signature", tx);
  });


  


  it("Initialize Collateral Config", async () => {
    const collateralVaultConfig = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("collateral"),
        collateralMint.toBuffer()
      ],
      program.programId
    )[0];
    const collateralVault = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vault"),
        collateralMint.toBuffer()
      ],
      program.programId
    )[0];

    // Add your test here.
    const tx = await program.methods.initializeCollateralVault(
      collateralPriceFeed
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
});
