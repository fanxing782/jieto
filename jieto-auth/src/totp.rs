use secrecy::{ExposeSecret, SecretBox};
use totp_rs::{Algorithm, Secret, TOTP};
use super::error::AuthError;

pub fn verify_totp(account_name: SecretBox<str>,digits:usize,skew:u8,step:u64,secret: SecretBox<str>, code:SecretBox<str>,) -> anyhow::Result<bool,AuthError> {
    let secret = Secret::Encoded(secret.expose_secret().to_string());
    let totp = TOTP::new(
        Algorithm::SHA1,
        digits,
        skew,
        step,
        secret.to_bytes()?,
        None,
        String::from(account_name.expose_secret())
    )?;
    // 验证一个 TOTP（允许 ±1 个时间窗口误差）
    let is_valid = totp.check_current(code.expose_secret())?;
    Ok(is_valid)
}

pub fn generate_totp_url(account_name: SecretBox<str>,issuer:Option<String>,digits:usize,skew:u8,step:u64,secret: SecretBox<str>) -> anyhow::Result<String,AuthError> {
    let secret = Secret::Encoded(secret.expose_secret().to_string());
    let totp = TOTP::new(
        Algorithm::SHA256,
        digits,
        skew,
        step,
        secret.to_bytes()?,
        issuer,
        String::from(account_name.expose_secret())
    )?;
    Ok(totp.get_url())
}
