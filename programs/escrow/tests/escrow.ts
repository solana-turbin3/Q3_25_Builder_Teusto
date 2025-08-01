import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Escrow } from "../target/types/escrow";
import { 
  PublicKey, 
  Keypair, 
  SystemProgram,
  LAMPORTS_PER_SOL 
} from "@solana/web3.js";
import { 
  TOKEN_PROGRAM_ID, 
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
  getAssociatedTokenAddress
} from "@solana/spl-token";
import { assert } from "chai";

describe("Escrow Program Tests", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Escrow as Program<Escrow>;
  
  // Test accounts - we'll create these in beforeEach
  let maker: Keypair;
  let taker: Keypair;
  let mintA: PublicKey; // Token the maker is offering (e.g., USDC)
  let mintB: PublicKey; // Token the maker wants (e.g., BONK)
  
  // Token accounts
  let makerAtaA: PublicKey; // Maker's account for mint A
  let makerAtaB: PublicKey; // Maker's account for mint B
  let takerAtaA: PublicKey; // Taker's account for mint A
  let takerAtaB: PublicKey; // Taker's account for mint B
  
  // Escrow accounts
  let escrow: PublicKey;
  let vault: PublicKey;
  
  // Test constants
  const seed = new anchor.BN(42); // Unique seed for this escrow
  const depositAmount = new anchor.BN(500_000_000); // 500 tokens (with 6 decimals)
  const receiveAmount = new anchor.BN(1_000_000_000); // 1000 tokens (with 6 decimals)
  
  console.log("🧪 Setting up comprehensive escrow tests...");
  
  beforeEach(async () => {
    console.log("\n🔄 Setting up fresh test environment...");
    
    // Step 1: Create fresh keypairs for each test
    maker = Keypair.generate();
    taker = Keypair.generate();
    
    console.log(`Maker: ${maker.publicKey.toString()}`);
    console.log(`Taker: ${taker.publicKey.toString()}`);
    
    // Step 2: Fund the accounts with SOL for transaction fees
    await provider.connection.requestAirdrop(maker.publicKey, 2 * LAMPORTS_PER_SOL);
    await provider.connection.requestAirdrop(taker.publicKey, 2 * LAMPORTS_PER_SOL);
    
    // Wait for airdrops to confirm
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // Step 3: Create two different token mints for testing
    mintA = await createMint(
      provider.connection,
      maker, // Payer
      maker.publicKey, // Mint authority
      null, // Freeze authority (none)
      6 // Decimals (like USDC)
    );
    
    mintB = await createMint(
      provider.connection,
      taker, // Payer
      taker.publicKey, // Mint authority
      null, // Freeze authority (none)
      6 // Decimals (like BONK)
    );
    
    console.log(`Mint A (maker's offering): ${mintA.toString()}`);
    console.log(`Mint B (maker wants): ${mintB.toString()}`);
    
    // Step 4: Create associated token accounts
    makerAtaA = await getAssociatedTokenAddress(mintA, maker.publicKey);
    makerAtaB = await getAssociatedTokenAddress(mintB, maker.publicKey);
    takerAtaA = await getAssociatedTokenAddress(mintA, taker.publicKey);
    takerAtaB = await getAssociatedTokenAddress(mintB, taker.publicKey);
    
    // Step 5: Create and fund token accounts
    // Maker gets mintA tokens (what they'll deposit)
    await createAccount(provider.connection, maker, mintA, maker.publicKey);
    await mintTo(
      provider.connection,
      maker, // Payer
      mintA, // Mint
      makerAtaA, // Destination
      maker, // Authority
      1000_000_000 // Amount: 1000 tokens
    );
    
    // Taker gets mintB tokens (what they'll pay with)
    await createAccount(provider.connection, taker, mintB, taker.publicKey);
    await mintTo(
      provider.connection,
      taker, // Payer
      mintB, // Mint
      takerAtaB, // Destination
      taker, // Authority
      2000_000_000 // Amount: 2000 tokens
    );
    
    // Step 6: Derive PDA addresses for escrow and vault
    [escrow] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow"),
        maker.publicKey.toBuffer(),
        seed.toArrayLike(Buffer, "le", 8)
      ],
      program.programId
    );
    
    vault = await getAssociatedTokenAddress(mintA, escrow, true); // true = allow PDA
    
    console.log(`Escrow PDA: ${escrow.toString()}`);
    console.log(`Vault ATA: ${vault.toString()}`);
    console.log("✅ Test environment ready!");
  });
  
  describe("Make Escrow Tests", () => {
    it("Should create escrow and deposit tokens successfully", async () => {
      console.log("\n🏗️  Testing make escrow...");
      
      // Get initial balances
      const initialMakerBalance = await getAccount(provider.connection, makerAtaA);
      console.log(`Initial maker balance: ${initialMakerBalance.amount} tokens`);
      
      // Call the make instruction
      const tx = await program.methods
        .make(
          seed,         // seed: u64
          receiveAmount, // receive: u64 (amount of mintB maker wants)
          depositAmount  // deposit: u64 (amount of mintA maker deposits)
        )
        .accounts({
          maker: maker.publicKey,
          mintA: mintA,
          mintB: mintB,
          makerAtaA: makerAtaA,
          escrow: escrow,
          vault: vault,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([maker])
        .rpc();
      
      console.log(`✅ Make transaction: ${tx}`);
      
      // Verify the escrow account was created with correct data
      const escrowAccount = await program.account.escrow.fetch(escrow);
      console.log("📋 Escrow account data:");
      console.log(`  Seed: ${escrowAccount.seed}`);
      console.log(`  Maker: ${escrowAccount.maker.toString()}`);
      console.log(`  Mint A: ${escrowAccount.mintA.toString()}`);
      console.log(`  Mint B: ${escrowAccount.mintB.toString()}`);
      console.log(`  Receive: ${escrowAccount.receive}`);
      
      // Assertions
      assert.equal(escrowAccount.seed.toString(), seed.toString());
      assert.equal(escrowAccount.maker.toString(), maker.publicKey.toString());
      assert.equal(escrowAccount.mintA.toString(), mintA.toString());
      assert.equal(escrowAccount.mintB.toString(), mintB.toString());
      assert.equal(escrowAccount.receive.toString(), receiveAmount.toString());
      
      // Verify tokens were transferred to vault
      const vaultAccount = await getAccount(provider.connection, vault);
      const finalMakerBalance = await getAccount(provider.connection, makerAtaA);
      
      console.log(`Vault balance: ${vaultAccount.amount} tokens`);
      console.log(`Final maker balance: ${finalMakerBalance.amount} tokens`);
      
      // Assertions for token transfer
      assert.equal(vaultAccount.amount.toString(), depositAmount.toString());
      assert.equal(
        finalMakerBalance.amount.toString(), 
        (BigInt(initialMakerBalance.amount.toString()) - BigInt(depositAmount.toString())).toString()
      );
      
      console.log("✅ Make escrow test passed!");
    });
  });
  
  describe("Take Escrow Tests", () => {
    beforeEach(async () => {
      // Create escrow first (needed for take tests)
      await program.methods
        .make(seed, receiveAmount, depositAmount)
        .accounts({
          maker: maker.publicKey,
          mintA: mintA,
          mintB: mintB,
          makerAtaA: makerAtaA,
          escrow: escrow,
          vault: vault,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([maker])
        .rpc();
      
      console.log("🔄 Escrow created for take tests");
    });
    
    it("Should fulfill escrow with atomic token swap", async () => {
      console.log("\n🔄 Testing take escrow (atomic swap)...");
      
      // Get initial balances for all parties
      const initialTakerBalanceB = await getAccount(provider.connection, takerAtaB);
      const initialVaultBalance = await getAccount(provider.connection, vault);
      
      console.log(`Initial taker mintB balance: ${initialTakerBalanceB.amount}`);
      console.log(`Initial vault balance: ${initialVaultBalance.amount}`);
      
      // Call the take instruction
      const tx = await program.methods
        .take()
        .accounts({
          taker: taker.publicKey,
          maker: maker.publicKey,
          mintA: mintA,
          mintB: mintB,
          takerAtaA: takerAtaA,
          takerAtaB: takerAtaB,
          makerAtaB: makerAtaB,
          escrow: escrow,
          vault: vault,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([taker])
        .rpc();
      
      console.log(`✅ Take transaction: ${tx}`);
      
      // Verify the atomic swap happened correctly
      console.log("🔍 Verifying atomic swap results...");
      
      // Check taker received mintA tokens (from vault)
      const finalTakerBalanceA = await getAccount(provider.connection, takerAtaA);
      console.log(`Taker received mintA: ${finalTakerBalanceA.amount}`);
      assert.equal(finalTakerBalanceA.amount.toString(), depositAmount.toString());
      
      // Check taker paid mintB tokens
      const finalTakerBalanceB = await getAccount(provider.connection, takerAtaB);
      const expectedTakerBalanceB = BigInt(initialTakerBalanceB.amount.toString()) - BigInt(receiveAmount.toString());
      console.log(`Taker paid mintB: ${receiveAmount} (remaining: ${finalTakerBalanceB.amount})`);
      assert.equal(finalTakerBalanceB.amount.toString(), expectedTakerBalanceB.toString());
      
      // Check maker received mintB tokens
      const finalMakerBalanceB = await getAccount(provider.connection, makerAtaB);
      console.log(`Maker received mintB: ${finalMakerBalanceB.amount}`);
      assert.equal(finalMakerBalanceB.amount.toString(), receiveAmount.toString());
      
      // Verify accounts were closed properly
      try {
        await program.account.escrow.fetch(escrow);
        assert.fail("Escrow account should be closed");
      } catch (error) {
        console.log("✅ Escrow account properly closed");
      }
      
      try {
        await getAccount(provider.connection, vault);
        assert.fail("Vault account should be closed");
      } catch (error) {
        console.log("✅ Vault account properly closed");
      }
      
      console.log("✅ Take escrow test passed! Atomic swap successful!");
    });
  });
  
  describe("Refund Escrow Tests", () => {
    beforeEach(async () => {
      // Create escrow first (needed for refund tests)
      await program.methods
        .make(seed, receiveAmount, depositAmount)
        .accounts({
          maker: maker.publicKey,
          mintA: mintA,
          mintB: mintB,
          makerAtaA: makerAtaA,
          escrow: escrow,
          vault: vault,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([maker])
        .rpc();
      
      console.log("🔄 Escrow created for refund tests");
    });
    
    it("Should refund tokens back to maker and close accounts", async () => {
      console.log("\n🔙 Testing refund escrow...");
      
      // Get initial balances
      const initialMakerBalance = await getAccount(provider.connection, makerAtaA);
      const initialVaultBalance = await getAccount(provider.connection, vault);
      
      console.log(`Initial maker balance: ${initialMakerBalance.amount}`);
      console.log(`Initial vault balance: ${initialVaultBalance.amount}`);
      
      // Call the refund instruction
      const tx = await program.methods
        .refund()
        .accounts({
          maker: maker.publicKey,
          mintA: mintA,
          makerAtaA: makerAtaA,
          escrow: escrow,
          vault: vault,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([maker])
        .rpc();
      
      console.log(`✅ Refund transaction: ${tx}`);
      
      // Verify tokens were returned to maker
      const finalMakerBalance = await getAccount(provider.connection, makerAtaA);
      const expectedMakerBalance = BigInt(initialMakerBalance.amount.toString()) + BigInt(initialVaultBalance.amount.toString());
      
      console.log(`Final maker balance: ${finalMakerBalance.amount}`);
      console.log(`Expected maker balance: ${expectedMakerBalance}`);
      
      assert.equal(finalMakerBalance.amount.toString(), expectedMakerBalance.toString());
      
      // Verify accounts were closed properly
      try {
        await program.account.escrow.fetch(escrow);
        assert.fail("Escrow account should be closed");
      } catch (error) {
        console.log("✅ Escrow account properly closed");
      }
      
      try {
        await getAccount(provider.connection, vault);
        assert.fail("Vault account should be closed");
      } catch (error) {
        console.log("✅ Vault account properly closed");
      }
      
      console.log("✅ Refund escrow test passed! Tokens returned successfully!");
    });
  });
  
  describe("Error Handling Tests", () => {
    beforeEach(async () => {
      // Create escrow for error tests
      await program.methods
        .make(seed, receiveAmount, depositAmount)
        .accounts({
          maker: maker.publicKey,
          mintA: mintA,
          mintB: mintB,
          makerAtaA: makerAtaA,
          escrow: escrow,
          vault: vault,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([maker])
        .rpc();
    });
    
    it("Should fail when wrong person tries to refund", async () => {
      console.log("\n⚠️  Testing unauthorized refund...");
      
      try {
        await program.methods
          .refund()
          .accounts({
            maker: taker.publicKey, // Wrong person!
            mintA: mintA,
            makerAtaA: await getAssociatedTokenAddress(mintA, taker.publicKey),
            escrow: escrow,
            vault: vault,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([taker])
          .rpc();
        
        assert.fail("Should have failed with unauthorized refund");
      } catch (error) {
        console.log(`✅ Correctly rejected unauthorized refund: ${error.message}`);
        assert(error.message.includes("has_one") || error.message.includes("ConstraintHasOne"));
      }
    });
    
    it("Should fail to take with insufficient balance", async () => {
      console.log("\n⚠️  Testing insufficient balance take...");
      
      // Create a new taker with insufficient tokens
      const poorTaker = Keypair.generate();
      await provider.connection.requestAirdrop(poorTaker.publicKey, LAMPORTS_PER_SOL);
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      const poorTakerAtaA = await getAssociatedTokenAddress(mintA, poorTaker.publicKey);
      const poorTakerAtaB = await getAssociatedTokenAddress(mintB, poorTaker.publicKey);
      
      // Give them some mintB tokens, but not enough
      await createAccount(provider.connection, poorTaker, mintB, poorTaker.publicKey);
      await mintTo(
        provider.connection,
        taker, // Original taker mints for poor taker
        mintB,
        poorTakerAtaB,
        taker,
        100_000_000 // Only 100 tokens, but needs 1000
      );
      
      try {
        await program.methods
          .take()
          .accounts({
            taker: poorTaker.publicKey,
            maker: maker.publicKey,
            mintA: mintA,
            mintB: mintB,
            takerAtaA: poorTakerAtaA,
            takerAtaB: poorTakerAtaB,
            makerAtaB: makerAtaB,
            escrow: escrow,
            vault: vault,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([poorTaker])
          .rpc();
        
        assert.fail("Should have failed with insufficient balance");
      } catch (error) {
        console.log(`✅ Correctly rejected insufficient balance: ${error.message}`);
        assert(error.message.includes("insufficient") || error.message.includes("InsufficientFunds"));
      }
    });
  });
});
