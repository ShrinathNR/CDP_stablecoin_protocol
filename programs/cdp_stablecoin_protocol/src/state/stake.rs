use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct StakeAccount {
    pub user: Pubkey,
    pub amount: u64,
    pub points: u64,
    pub last_staked: i64,
    pub bump: u8,
}

#[account]
pub struct StakerRegistry {
    pub stake_accounts: Vec<StakeAccount>,
    // pub total_stake_points: u64,
}


impl StakerRegistry {
    pub fn update_stake_account(&mut self, stake_account: StakeAccount) {
        if let Some(index) = self.stake_accounts.iter().position(|x| x.user == stake_account.user) {
            self.stake_accounts[index] = stake_account;
        } else {
            self.stake_accounts.push(stake_account);
        }
    }

    pub fn remove_stake_account(&mut self, user: Pubkey) {
        self.stake_accounts.retain(|x| x.user != user);
    }
}