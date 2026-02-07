use anchor_lang::prelude::*;

declare_id!("ADoo99CXxaaidHu9NAqT8jMEEHJLW8cCKvgwTzAbgW3m");

#[program]
pub mod solana_voting {
    use super::*;

    /// Inicializa el contrato/programa:
    /// - crea Config PDA
    /// - setea owner = signer
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.owner = ctx.accounts.owner.key();
        config.total_proposals = 0;
        Ok(())
    }

    /// Solo owner puede crear propuestas
    /// Retorna el ID incremental
    pub fn create_proposal(ctx: Context<CreateProposal>, title: String) -> Result<u32> {
        require!(
            title.as_bytes().len() <= Proposal::MAX_TITLE_LEN,
            VotingError::TitleTooLong
        );

        let config = &mut ctx.accounts.config;
        let owner = &ctx.accounts.owner;

        require!(config.owner == owner.key(), VotingError::NotOwner);

        let id = config.total_proposals; // ID incremental
        config.total_proposals = config
            .total_proposals
            .checked_add(1)
            .ok_or(VotingError::MathOverflow)?;

        let proposal = &mut ctx.accounts.proposal;
        proposal.id = id;
        proposal.title = title.clone();
        proposal.votes = 0;

        emit!(ProposalCreated { id, title });

        Ok(id)
    }

    /// Votar una sola vez por propuesta
    pub fn vote(ctx: Context<Vote>, proposal_id: u32) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;

        // Validación extra: por si alguien intenta pasar un proposal que no coincide
        require!(proposal.id == proposal_id, VotingError::InvalidProposalAccount);

        // Si VoteRecord se pudo crear, es porque NO existía antes => no votó todavía.
        let vote_record = &mut ctx.accounts.vote_record;
        vote_record.proposal_id = proposal_id;
        vote_record.voter = ctx.accounts.voter.key();

        proposal.votes = proposal
            .votes
            .checked_add(1)
            .ok_or(VotingError::MathOverflow)?;

        emit!(VoteCast {
            proposal_id,
            voter: ctx.accounts.voter.key()
        });

        Ok(())
    }

    /// Consulta: devuelve (title, votes) para una propuesta
    pub fn get_proposal(ctx: Context<GetProposal>) -> Result<(String, u32)> {
        let proposal = &ctx.accounts.proposal;
        Ok((proposal.title.clone(), proposal.votes))
    }

    /// Consulta: total de propuestas
    pub fn total_proposals(ctx: Context<TotalProposals>) -> Result<u32> {
        Ok(ctx.accounts.config.total_proposals)
    }
}

/* ----------------------------- ACCOUNTS ----------------------------- */

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + Config::SIZE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateProposal<'info> {
    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,

    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = 8 + Proposal::space_for_title(), // espacio dinámico para String
        seeds = [b"proposal", config.total_proposals.to_le_bytes().as_ref()],
        bump
    )]
    pub proposal: Account<'info, Proposal>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposal_id: u32)]
pub struct Vote<'info> {
    #[account(
        seeds = [b"proposal", proposal_id.to_le_bytes().as_ref()],
        bump,
        mut
    )]
    pub proposal: Account<'info, Proposal>,

    #[account(mut)]
    pub voter: Signer<'info>,

    // Si ya existe, init falla => DoubleVote (lo capturamos en tests, o con constraint opcional)
    #[account(
        init,
        payer = voter,
        space = 8 + VoteRecord::SIZE,
        seeds = [b"vote", proposal_id.to_le_bytes().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub vote_record: Account<'info, VoteRecord>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposal_id: u32)]
pub struct GetProposal<'info> {
    #[account(
        seeds = [b"proposal", proposal_id.to_le_bytes().as_ref()],
        bump
    )]
    pub proposal: Account<'info, Proposal>,
}

#[derive(Accounts)]
pub struct TotalProposals<'info> {
    #[account(
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,
}

/* ------------------------------ STATE ------------------------------ */

#[account]
pub struct Config {
    pub owner: Pubkey,
    pub total_proposals: u32,
}
impl Config {
    pub const SIZE: usize = 32 + 4;
}

#[account]
pub struct Proposal {
    pub id: u32,
    pub votes: u32,
    pub title: String, // Anchor: 4 bytes length + bytes
}
impl Proposal {
    pub const MAX_TITLE_LEN: usize = 64;

    /// 4(id) + 4(votes) + 4(len) + MAX_TITLE_LEN
    pub fn space_for_title() -> usize {
        4 + 4 + 4 + Self::MAX_TITLE_LEN
    }
}

#[account]
pub struct VoteRecord {
    pub proposal_id: u32,
    pub voter: Pubkey,
}
impl VoteRecord {
    pub const SIZE: usize = 4 + 32;
}

/* ------------------------------ EVENTS ----------------------------- */

#[event]
pub struct ProposalCreated {
    pub id: u32,
    pub title: String,
}

#[event]
pub struct VoteCast {
    pub proposal_id: u32,
    pub voter: Pubkey,
}

/* ------------------------------ ERRORS ----------------------------- */

#[error_code]
pub enum VotingError {
    #[msg("Only the owner can create proposals.")]
    NotOwner,
    #[msg("Proposal title is too long.")]
    TitleTooLong,
    #[msg("Math overflow.")]
    MathOverflow,
    #[msg("Invalid proposal account for given proposal_id.")]
    InvalidProposalAccount,
}
