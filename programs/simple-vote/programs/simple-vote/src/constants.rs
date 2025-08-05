// PDA Seeds for deterministic address generation

// Seed for Poll PDAs: ["poll", creator.key(), poll_id]
// This allows each creator to have multiple polls with unique IDs
pub const POLL_SEED: &[u8] = b"poll";

// Seed for Vote Receipt PDAs: ["vote", poll.key(), voter.key()]
// This ensures one vote receipt per voter per poll
pub const VOTE_SEED: &[u8] = b"vote";

// Maximum values for validation
pub const MAX_QUESTION_LENGTH: usize = 200;
pub const MAX_OPTION_LENGTH: usize = 50;
pub const MAX_OPTIONS_COUNT: usize = 10;

// Minimum poll duration (1 hour in seconds)
pub const MIN_POLL_DURATION: i64 = 3600;

// Maximum poll duration (30 days in seconds)
pub const MAX_POLL_DURATION: i64 = 30 * 24 * 3600;

// Anchor discriminator size (8 bytes)
pub const DISCRIMINATOR_SIZE: usize = 8;