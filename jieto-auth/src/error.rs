use thiserror::Error;
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("'{0}'")]
    SystemTimeError(#[from] std::time::SystemTimeError),
    #[cfg(feature = "totp")]
    #[error("'{0}'")]
    TotpUrlError(#[from] totp_rs::TotpUrlError),
    #[cfg(feature = "totp")]
    #[error("'{0}'")]
    SecretParseError(#[from] totp_rs::SecretParseError)
}
