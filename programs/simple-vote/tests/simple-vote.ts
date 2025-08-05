import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SimpleVote } from "../target/types/simple_vote";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";

describe("Simple Vote System Tests", () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider();
  const program = anchor.workspace.SimpleVote as Program<SimpleVote>;

  // Test accounts - we'll create fresh ones for each test
  let creator: Keypair;
  let voter1: Keypair;
  let voter2: Keypair;
  let voter3: Keypair;

  // Poll configuration
  let pollId: number;
  let question: string;
  let options: string[];
  let durationSeconds: number;

  // Derived addresses
  let pollPda: PublicKey;
  let pollBump: number;

  console.log("🗳️  Starting Simple Vote System Tests");
  console.log("Program ID:", program.programId.toString());

  beforeEach(async () => {
    console.log("\n🔄 Setting up fresh test environment...");
    
    // Create fresh keypairs for each test
    creator = Keypair.generate();
    voter1 = Keypair.generate();
    voter2 = Keypair.generate();
    voter3 = Keypair.generate();

    console.log("👤 Creator:", creator.publicKey.toString());
    console.log("🗳️  Voter 1:", voter1.publicKey.toString());
    console.log("🗳️  Voter 2:", voter2.publicKey.toString());
    console.log("🗳️  Voter 3:", voter3.publicKey.toString());

    // Fund all accounts with SOL
    const accounts = [creator, voter1, voter2, voter3];
    for (const account of accounts) {
      const signature = await provider.connection.requestAirdrop(
        account.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(signature);
    }

    // Set up poll parameters
    pollId = Math.floor(Math.random() * 1000000); // Random poll ID
    question = "What's your favorite programming language?";
    options = ["Rust", "TypeScript", "Python", "Go"];
    durationSeconds = 3600; // 1 hour

    // Derive poll PDA - must match Rust: poll_id.to_le_bytes().as_ref()
    const pollIdBuffer = Buffer.allocUnsafe(8);
    pollIdBuffer.writeBigUInt64LE(BigInt(pollId), 0);
    
    [pollPda, pollBump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("poll"),
        creator.publicKey.toBuffer(),
        pollIdBuffer, // u64 as 8 bytes little-endian
      ],
      program.programId
    );

    console.log("📊 Poll ID:", pollId);
    console.log("❓ Question:", question);
    console.log("📝 Options:", options.join(", "));
    console.log("🏠 Poll PDA:", pollPda.toString());
    console.log("⏰ Duration:", durationSeconds, "seconds");

    // Wait a moment to ensure all airdrops are confirmed
    await new Promise(resolve => setTimeout(resolve, 1000));
  });

  describe("Poll Creation Tests", () => {
    it("✅ Should create a poll successfully", async () => {
      console.log("\n🧪 Testing: Create Poll");
      
      const tx = await program.methods
        .createPoll(
          new anchor.BN(pollId),
          question,
          options,
          new anchor.BN(durationSeconds)
        )
        .accounts({
          creator: creator.publicKey,
          poll: pollPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([creator])
        .rpc();

      console.log("✅ Poll created! Transaction:", tx);

      // Fetch and verify poll data
      const pollAccount = await program.account.poll.fetch(pollPda);
      
      console.log("📊 Poll Data:");
      console.log("  Creator:", pollAccount.creator.toString());
      console.log("  Poll ID:", pollAccount.pollId.toString());
      console.log("  Question:", pollAccount.question);
      console.log("  Options:", pollAccount.options);
      console.log("  Vote Counts:", pollAccount.voteCounts.map(v => v.toString()));
      console.log("  Is Active:", pollAccount.isActive);
      console.log("  Total Votes:", pollAccount.totalVotes.toString());

      // Assertions
      assert.equal(pollAccount.creator.toString(), creator.publicKey.toString());
      assert.equal(pollAccount.pollId.toString(), pollId.toString());
      assert.equal(pollAccount.question, question);
      assert.deepEqual(pollAccount.options, options);
      assert.equal(pollAccount.voteCounts.length, options.length);
      assert.isTrue(pollAccount.isActive);
      assert.equal(pollAccount.totalVotes.toString(), "0");
      
      // All vote counts should start at 0
      pollAccount.voteCounts.forEach((count, index) => {
        assert.equal(count.toString(), "0", `Option ${index} should start with 0 votes`);
      });
    });

    it("❌ Should fail with question too long", async () => {
      console.log("\n🧪 Testing: Question Too Long Error");
      
      const longQuestion = "A".repeat(201); // Exceeds 200 character limit
      
      try {
        await program.methods
          .createPoll(
            new anchor.BN(pollId),
            longQuestion,
            options,
            new anchor.BN(durationSeconds)
          )
          .accounts({
            creator: creator.publicKey,
            poll: pollPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([creator])
          .rpc();
        
        assert.fail("Should have failed with question too long");
      } catch (error) {
        console.log("✅ Correctly failed with error:", error.message);
        assert(error.message.includes("QuestionTooLong"));
      }
    });

    it("❌ Should fail with too few options", async () => {
      console.log("\n🧪 Testing: Too Few Options Error");
      
      const tooFewOptions = ["Only One Option"];
      
      try {
        await program.methods
          .createPoll(
            new anchor.BN(pollId),
            question,
            tooFewOptions,
            new anchor.BN(durationSeconds)
          )
          .accounts({
            creator: creator.publicKey,
            poll: pollPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([creator])
          .rpc();
        
        assert.fail("Should have failed with too few options");
      } catch (error) {
        console.log("✅ Correctly failed with error:", error.message);
        assert(error.message.includes("NotEnoughOptions"));
      }
    });

    it("❌ Should fail with duration too short", async () => {
      console.log("\n🧪 Testing: Duration Too Short Error");
      
      const shortDuration = 1800; // 30 minutes (less than 1 hour minimum)
      
      try {
        await program.methods
          .createPoll(
            new anchor.BN(pollId),
            question,
            options,
            new anchor.BN(shortDuration)
          )
          .accounts({
            creator: creator.publicKey,
            poll: pollPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([creator])
          .rpc();
        
        assert.fail("Should have failed with duration too short");
      } catch (error) {
        console.log("✅ Correctly failed with error:", error.message);
        assert(error.message.includes("PollDurationTooShort"));
      }
    });
  });

  describe("Voting Tests", () => {
    beforeEach(async () => {
      // Create a poll before each voting test
      await program.methods
        .createPoll(
          new anchor.BN(pollId),
          question,
          options,
          new anchor.BN(durationSeconds)
        )
        .accounts({
          creator: creator.publicKey,
          poll: pollPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([creator])
        .rpc();
    });

    it("✅ Should cast vote successfully", async () => {
      console.log("\n🧪 Testing: Cast Vote");
      
      const optionIndex = 0; // Vote for "Rust"
      
      // Derive vote receipt PDA
      const [voteReceiptPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote"),
          pollPda.toBuffer(),
          voter1.publicKey.toBuffer(),
        ],
        program.programId
      );

      console.log("🎫 Vote Receipt PDA:", voteReceiptPda.toString());
      console.log("🗳️  Voting for option", optionIndex, ":", options[optionIndex]);

      const tx = await program.methods
        .castVote(optionIndex)
        .accounts({
          voter: voter1.publicKey,
          poll: pollPda,
          voteReceipt: voteReceiptPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([voter1])
        .rpc();

      console.log("✅ Vote cast! Transaction:", tx);

      // Verify poll was updated
      const pollAccount = await program.account.poll.fetch(pollPda);
      console.log("📊 Updated vote counts:", pollAccount.voteCounts.map(v => v.toString()));
      console.log("📈 Total votes:", pollAccount.totalVotes.toString());

      assert.equal(pollAccount.voteCounts[optionIndex].toString(), "1");
      assert.equal(pollAccount.totalVotes.toString(), "1");

      // Verify vote receipt was created
      const voteReceipt = await program.account.voteReceipt.fetch(voteReceiptPda);
      console.log("🎫 Vote Receipt:");
      console.log("  Poll:", voteReceipt.poll.toString());
      console.log("  Voter:", voteReceipt.voter.toString());
      console.log("  Option Index:", voteReceipt.optionIndex);
      console.log("  Voted At:", new Date(voteReceipt.votedAt.toNumber() * 1000));

      assert.equal(voteReceipt.poll.toString(), pollPda.toString());
      assert.equal(voteReceipt.voter.toString(), voter1.publicKey.toString());
      assert.equal(voteReceipt.optionIndex, optionIndex);
    });

    it("✅ Should handle multiple votes correctly", async () => {
      console.log("\n🧪 Testing: Multiple Votes");
      
      // Cast votes from different voters
      const votes = [
        { voter: voter1, option: 0 }, // Rust
        { voter: voter2, option: 1 }, // TypeScript
        { voter: voter3, option: 0 }, // Rust (again)
      ];

      for (const vote of votes) {
        const [voteReceiptPda] = PublicKey.findProgramAddressSync(
          [
            Buffer.from("vote"),
            pollPda.toBuffer(),
            vote.voter.publicKey.toBuffer(),
          ],
          program.programId
        );

        console.log(`🗳️  ${vote.voter.publicKey.toString().slice(0, 8)}... voting for option ${vote.option}: ${options[vote.option]}`);

        await program.methods
          .castVote(vote.option)
          .accounts({
            voter: vote.voter.publicKey,
            poll: pollPda,
            voteReceipt: voteReceiptPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([vote.voter])
          .rpc();
      }

      // Verify final vote counts
      const pollAccount = await program.account.poll.fetch(pollPda);
      console.log("📊 Final vote counts:", pollAccount.voteCounts.map(v => v.toString()));
      console.log("📈 Total votes:", pollAccount.totalVotes.toString());

      // Rust should have 2 votes, TypeScript should have 1
      assert.equal(pollAccount.voteCounts[0].toString(), "2"); // Rust
      assert.equal(pollAccount.voteCounts[1].toString(), "1"); // TypeScript
      assert.equal(pollAccount.voteCounts[2].toString(), "0"); // Python
      assert.equal(pollAccount.voteCounts[3].toString(), "0"); // Go
      assert.equal(pollAccount.totalVotes.toString(), "3");
    });

    it("❌ Should prevent double voting", async () => {
      console.log("\n🧪 Testing: Double Voting Prevention");
      
      const optionIndex = 0;
      const [voteReceiptPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote"),
          pollPda.toBuffer(),
          voter1.publicKey.toBuffer(),
        ],
        program.programId
      );

      // First vote should succeed
      await program.methods
        .castVote(optionIndex)
        .accounts({
          voter: voter1.publicKey,
          poll: pollPda,
          voteReceipt: voteReceiptPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([voter1])
        .rpc();

      console.log("✅ First vote successful");

      // Second vote should fail
      try {
        await program.methods
          .castVote(1) // Try to vote for different option
          .accounts({
            voter: voter1.publicKey,
            poll: pollPda,
            voteReceipt: voteReceiptPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([voter1])
          .rpc();
        
        assert.fail("Should have failed with double voting");
      } catch (error) {
        console.log("✅ Correctly prevented double voting:", error.message);
        // The error will be about account already exists (vote receipt PDA)
        assert(error.message.includes("already in use") || error.message.includes("AlreadyInUse"));
      }
    });

    it("❌ Should fail with invalid option index", async () => {
      console.log("\n🧪 Testing: Invalid Option Index");
      
      const invalidOptionIndex = 99; // Way out of bounds
      const [voteReceiptPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote"),
          pollPda.toBuffer(),
          voter1.publicKey.toBuffer(),
        ],
        program.programId
      );

      try {
        await program.methods
          .castVote(invalidOptionIndex)
          .accounts({
            voter: voter1.publicKey,
            poll: pollPda,
            voteReceipt: voteReceiptPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([voter1])
          .rpc();
        
        assert.fail("Should have failed with invalid option");
      } catch (error) {
        console.log("✅ Correctly failed with invalid option:", error.message);
        assert(error.message.includes("InvalidOption"));
      }
    });
  });

  describe("Poll Management Tests", () => {
    beforeEach(async () => {
      // Create a poll before each test
      await program.methods
        .createPoll(
          new anchor.BN(pollId),
          question,
          options,
          new anchor.BN(durationSeconds)
        )
        .accounts({
          creator: creator.publicKey,
          poll: pollPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([creator])
        .rpc();
    });

    it("✅ Should close poll successfully", async () => {
      console.log("\n🧪 Testing: Close Poll");
      
      // First, let's cast some votes to make it interesting
      const [voteReceiptPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote"),
          pollPda.toBuffer(),
          voter1.publicKey.toBuffer(),
        ],
        program.programId
      );

      await program.methods
        .castVote(0) // Vote for Rust
        .accounts({
          voter: voter1.publicKey,
          poll: pollPda,
          voteReceipt: voteReceiptPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([voter1])
        .rpc();

      console.log("✅ Vote cast before closing");

      // Now close the poll
      const tx = await program.methods
        .closePoll()
        .accounts({
          creator: creator.publicKey,
          poll: pollPda,
        })
        .signers([creator])
        .rpc();

      console.log("✅ Poll closed! Transaction:", tx);

      // Verify poll is closed
      const pollAccount = await program.account.poll.fetch(pollPda);
      console.log("📊 Poll after closing:");
      console.log("  Is Active:", pollAccount.isActive);
      console.log("  Total Votes:", pollAccount.totalVotes.toString());
      console.log("  Vote Counts:", pollAccount.voteCounts.map(v => v.toString()));

      assert.isFalse(pollAccount.isActive);
    });

    it("❌ Should prevent non-creator from closing poll", async () => {
      console.log("\n🧪 Testing: Unauthorized Poll Closure");
      
      try {
        await program.methods
          .closePoll()
          .accounts({
            creator: voter1.publicKey, // Wrong creator!
            poll: pollPda,
          })
          .signers([voter1])
          .rpc();
        
        assert.fail("Should have failed with unauthorized creator");
      } catch (error) {
        console.log("✅ Correctly prevented unauthorized closure:", error.message);
        assert(error.message.includes("has_one") || error.message.includes("ConstraintHasOne"));
      }
    });

    it("❌ Should prevent voting on closed poll", async () => {
      console.log("\n🧪 Testing: Voting on Closed Poll");
      
      // Close the poll first
      await program.methods
        .closePoll()
        .accounts({
          creator: creator.publicKey,
          poll: pollPda,
        })
        .signers([creator])
        .rpc();

      console.log("✅ Poll closed");

      // Try to vote on closed poll
      const [voteReceiptPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote"),
          pollPda.toBuffer(),
          voter1.publicKey.toBuffer(),
        ],
        program.programId
      );

      try {
        await program.methods
          .castVote(0)
          .accounts({
            voter: voter1.publicKey,
            poll: pollPda,
            voteReceipt: voteReceiptPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([voter1])
          .rpc();
        
        assert.fail("Should have failed voting on closed poll");
      } catch (error) {
        console.log("✅ Correctly prevented voting on closed poll:", error.message);
        assert(error.message.includes("PollNotActive"));
      }
    });
  });

  describe("Integration Tests", () => {
    it("🎯 Complete voting scenario", async () => {
      console.log("\n🧪 Testing: Complete Voting Scenario");
      
      // 1. Create poll
      console.log("📋 Step 1: Creating poll...");
      await program.methods
        .createPoll(
          new anchor.BN(pollId),
          "Which blockchain is best for DeFi?",
          ["Solana", "Ethereum", "Polygon", "Avalanche"],
          new anchor.BN(7200) // 2 hours
        )
        .accounts({
          creator: creator.publicKey,
          poll: pollPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([creator])
        .rpc();
      
      console.log("✅ Poll created successfully");

      // 2. Multiple users vote
      console.log("🗳️  Step 2: Casting votes...");
      const voters = [voter1, voter2, voter3];
      const voteChoices = [0, 0, 1]; // Two votes for Solana, one for Ethereum
      
      for (let i = 0; i < voters.length; i++) {
        const [voteReceiptPda] = PublicKey.findProgramAddressSync(
          [
            Buffer.from("vote"),
            pollPda.toBuffer(),
            voters[i].publicKey.toBuffer(),
          ],
          program.programId
        );

        await program.methods
          .castVote(voteChoices[i])
          .accounts({
            voter: voters[i].publicKey,
            poll: pollPda,
            voteReceipt: voteReceiptPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([voters[i]])
          .rpc();
        
        console.log(`  ✅ Vote ${i + 1} cast successfully`);
      }

      // 3. Check intermediate results
      console.log("📊 Step 3: Checking results...");
      const pollAccount = await program.account.poll.fetch(pollPda);
      console.log("  Current vote counts:", pollAccount.voteCounts.map(v => v.toString()));
      console.log("  Total votes:", pollAccount.totalVotes.toString());
      
      // Verify vote distribution
      assert.equal(pollAccount.voteCounts[0].toString(), "2"); // Solana
      assert.equal(pollAccount.voteCounts[1].toString(), "1"); // Ethereum
      assert.equal(pollAccount.voteCounts[2].toString(), "0"); // Polygon
      assert.equal(pollAccount.voteCounts[3].toString(), "0"); // Avalanche
      assert.equal(pollAccount.totalVotes.toString(), "3");

      // 4. Close poll and announce winner
      console.log("🏁 Step 4: Closing poll...");
      await program.methods
        .closePoll()
        .accounts({
          creator: creator.publicKey,
          poll: pollPda,
        })
        .signers([creator])
        .rpc();
      
      console.log("✅ Poll closed successfully");
      
      // 5. Verify final state
      const finalPollAccount = await program.account.poll.fetch(pollPda);
      assert.isFalse(finalPollAccount.isActive);
      
      console.log("🏆 Final Results:");
      finalPollAccount.options.forEach((option, index) => {
        const votes = finalPollAccount.voteCounts[index].toString();
        console.log(`  ${option}: ${votes} votes`);
      });
      
      console.log("🎉 Complete voting scenario successful!");
    });
  });
});
