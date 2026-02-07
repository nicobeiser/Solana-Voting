import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaVoting } from "../target/types/solana_voting";
import { assert } from "chai";

describe("solana-voting", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaVoting as Program<SolanaVoting>;
  const owner = provider.wallet as anchor.Wallet;

  const voter = anchor.web3.Keypair.generate();

  // helper: u32 little-endian (4 bytes)
  const u32le = (n: number) => new anchor.BN(n).toArrayLike(Buffer, "le", 4);

  const [configPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    program.programId
  );

  it("Airdrop voter", async () => {
    const sig = await provider.connection.requestAirdrop(voter.publicKey, 2e9);
    await provider.connection.confirmTransaction(sig);
  });

  it("Initialize", async () => {
    await program.methods
      .initialize()
      .accountsPartial({
        config: configPda,
        owner: owner.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId, 
      })
      .rpc();

    const cfg = await program.account.config.fetch(configPda);
    assert.equal(cfg.owner.toBase58(), owner.publicKey.toBase58());
    assert.equal(cfg.totalProposals, 0);
  });

  it("Owner creates proposal", async () => {
    const proposalId = 0;

    const [proposalPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("proposal"), u32le(proposalId)],
      program.programId
    );

    await program.methods
      .createProposal("Primera")
      .accountsPartial({
        config: configPda,
        owner: owner.publicKey,
        proposal: proposalPda,
        systemProgram: anchor.web3.SystemProgram.programId, 
      })
      .rpc();

    const proposal = await program.account.proposal.fetch(proposalPda);
    assert.equal(proposal.id, proposalId);
    assert.equal(proposal.votes, 0);
    assert.equal(proposal.title, "Primera");

    const cfg = await program.account.config.fetch(configPda);
    assert.equal(cfg.totalProposals, 1);
  });

  it("Non-owner cannot create proposal", async () => {
    const fakeOwner = voter;

    const cfg = await program.account.config.fetch(configPda);
    const nextId = cfg.totalProposals as number;

    const [proposalPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("proposal"), u32le(nextId)],
      program.programId
    );

    try {
      await program.methods
        .createProposal("Hack")
        .accountsPartial({
          config: configPda,
          owner: fakeOwner.publicKey,
          proposal: proposalPda,
          systemProgram: anchor.web3.SystemProgram.programId, 
        })
        .signers([fakeOwner])
        .rpc();

      assert.fail("Should have failed");
    } catch (e: any) {
      assert.include(e.toString(), "Only the owner can create proposals");
    }
  });

  it("Vote once", async () => {
    const proposalId = 0;

    const [proposalPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("proposal"), u32le(proposalId)],
      program.programId
    );

    const [votePda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vote"), u32le(proposalId), voter.publicKey.toBuffer()],
      program.programId
    );

    await program.methods
      .vote(proposalId)
      .accountsPartial({
        proposal: proposalPda,
        voter: voter.publicKey,
        voteRecord: votePda, 
        systemProgram: anchor.web3.SystemProgram.programId, 
      })
      .signers([voter])
      .rpc();

    const proposal = await program.account.proposal.fetch(proposalPda);
    assert.equal(proposal.votes, 1);
  });

  it("Cannot vote twice", async () => {
    const proposalId = 0;

    const [proposalPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("proposal"), u32le(proposalId)],
      program.programId
    );

    const [votePda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vote"), u32le(proposalId), voter.publicKey.toBuffer()],
      program.programId
    );

    try {
      await program.methods
        .vote(proposalId)
        .accountsPartial({
          proposal: proposalPda,
          voter: voter.publicKey,
          voteRecord: votePda, 
          systemProgram: anchor.web3.SystemProgram.programId, 
        })
        .signers([voter])
        .rpc();

      assert.fail("Should have failed");
    } catch (e: any) {

      const msg = e.toString().toLowerCase();
      assert.isTrue(msg.includes("already") || msg.includes("initialized") || msg.includes("in use"));
    }
  });
});
