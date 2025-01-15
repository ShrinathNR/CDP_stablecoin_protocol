use anchor_lang::error_code;

#[error_code]
pub enum PositionError {
    #[msg("Interest Rate Should Be Between 1% And 100%")]
    InvalidInterestRate,
    #[msg("LTV Should Be Between 1% And 80%")]
    InvalidLTV,
}
#[error_code]
pub enum CollateralError {
    #[msg("This Mint Is Not Supported By The Protocol")]
    InvalidMintAsCollateral,
}

#[error_code]
pub enum ArithmeticError {
    #[msg("Arithmetic Overflow")]
    ArithmeticOverflow,
}
