use anchor_lang::prelude::*;
use crate::constants::*;

/// Utility functions for PDA derivation and validation
pub struct PdaUtils;

impl PdaUtils {
    /// Derive policy config PDA
    pub fn derive_policy_config_pda(
        program_id: &Pubkey,
        vault: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[POLICY_SEED, vault.as_ref()],
            program_id,
        )
    }

    /// Derive distribution progress PDA
    pub fn derive_distribution_progress_pda(
        program_id: &Pubkey,
        vault: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[PROGRESS_SEED, vault.as_ref()],
            program_id,
        )
    }

    /// Derive position owner PDA
    pub fn derive_position_owner_pda(
        program_id: &Pubkey,
        vault: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[VAULT_SEED, vault.as_ref(), b"investor_fee_pos_owner"],
            program_id,
        )
    }

    /// Derive treasury ATA PDA (if using program-owned treasury)
    pub fn derive_treasury_ata_pda(
        program_id: &Pubkey,
        vault: &Pubkey,
        quote_mint: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[VAULT_SEED, vault.as_ref(), b"treasury", quote_mint.as_ref()],
            program_id,
        )
    }

    /// Validate that a PDA matches expected derivation
    pub fn validate_policy_config_pda(
        program_id: &Pubkey,
        vault: &Pubkey,
        pda: &Pubkey,
        bump: u8,
    ) -> bool {
        let (expected_pda, expected_bump) = Self::derive_policy_config_pda(program_id, vault);
        expected_pda == *pda && expected_bump == bump
    }

    /// Validate that a distribution progress PDA matches expected derivation
    pub fn validate_distribution_progress_pda(
        program_id: &Pubkey,
        vault: &Pubkey,
        pda: &Pubkey,
        bump: u8,
    ) -> bool {
        let (expected_pda, expected_bump) = Self::derive_distribution_progress_pda(program_id, vault);
        expected_pda == *pda && expected_bump == bump
    }

    /// Validate that a position owner PDA matches expected derivation
    pub fn validate_position_owner_pda(
        program_id: &Pubkey,
        vault: &Pubkey,
        pda: &Pubkey,
        bump: u8,
    ) -> bool {
        let (expected_pda, expected_bump) = Self::derive_position_owner_pda(program_id, vault);
        expected_pda == *pda && expected_bump == bump
    }

    /// Get seeds for signing with policy config PDA
    pub fn get_policy_config_signer_seeds<'a>(
        vault: &'a Pubkey,
        bump: &'a [u8; 1],
    ) -> [&'a [u8]; 3] {
        [POLICY_SEED, vault.as_ref(), bump]
    }

    /// Get seeds for signing with distribution progress PDA
    pub fn get_distribution_progress_signer_seeds<'a>(
        vault: &'a Pubkey,
        bump: &'a [u8; 1],
    ) -> [&'a [u8]; 3] {
        [PROGRESS_SEED, vault.as_ref(), bump]
    }

    /// Get seeds for signing with position owner PDA
    pub fn get_position_owner_signer_seeds<'a>(
        vault: &'a Pubkey,
        bump: &'a [u8; 1],
    ) -> [&'a [u8]; 4] {
        [VAULT_SEED, vault.as_ref(), b"investor_fee_pos_owner", bump]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pda_derivation_consistency() {
        let program_id = Pubkey::new_unique();
        let vault = Pubkey::new_unique();

        // Test policy config PDA
        let (pda1, bump1) = PdaUtils::derive_policy_config_pda(&program_id, &vault);
        let (pda2, bump2) = PdaUtils::derive_policy_config_pda(&program_id, &vault);
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);

        // Test validation
        assert!(PdaUtils::validate_policy_config_pda(&program_id, &vault, &pda1, bump1));
        assert!(!PdaUtils::validate_policy_config_pda(&program_id, &vault, &pda1, bump1.wrapping_add(1)));
    }

    #[test]
    fn test_distribution_progress_pda() {
        let program_id = Pubkey::new_unique();
        let vault = Pubkey::new_unique();

        let (pda, bump) = PdaUtils::derive_distribution_progress_pda(&program_id, &vault);
        assert!(PdaUtils::validate_distribution_progress_pda(&program_id, &vault, &pda, bump));
    }

    #[test]
    fn test_position_owner_pda() {
        let program_id = Pubkey::new_unique();
        let vault = Pubkey::new_unique();

        let (pda, bump) = PdaUtils::derive_position_owner_pda(&program_id, &vault);
        assert!(PdaUtils::validate_position_owner_pda(&program_id, &vault, &pda, bump));
    }

    #[test]
    fn test_signer_seeds() {
        let vault = Pubkey::new_unique();
        let bump = [255u8];

        let seeds = PdaUtils::get_policy_config_signer_seeds(&vault, &bump);
        assert_eq!(seeds[0], POLICY_SEED);
        assert_eq!(seeds[1], vault.as_ref());
        assert_eq!(seeds[2], &bump);
    }
}