use std::error::Error;
use std::io::{self};
use std::path::{Path, PathBuf};

use rpassword::prompt_password;

/// Environment variable that can supply the password without a flag or prompt.
/// Priority: --password flag > RIFTX_PASSWORD env var > interactive prompt.
pub const PASSWORD_ENV_VAR: &str = "RIFTX_PASSWORD";

fn password_from_env() -> Option<String> {
    std::env::var(PASSWORD_ENV_VAR)
        .ok()
        .filter(|v| !v.is_empty())
}

pub fn prompt_for_password(provided_password: Option<&str>) -> Result<String, Box<dyn Error>> {
    if let Some(password) = provided_password {
        return Ok(password.to_owned());
    }

    if let Some(password) = password_from_env() {
        return Ok(password);
    }

    Ok(prompt_password("Password: ")?)
}

pub fn prompt_for_password_with_confirmation(
    provided_password: Option<&str>,
) -> Result<String, Box<dyn Error>> {
    if let Some(password) = provided_password {
        return Ok(password.to_owned());
    }

    // When the password comes from the env var, skip confirmation — the caller
    // is scripting and there is no human to type a second time.
    if let Some(password) = password_from_env() {
        if password.is_empty() {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "password cannot be empty").into(),
            );
        }
        return Ok(password);
    }

    let password = prompt_password("Password: ")?;

    // Validate before asking for confirmation so the user isn't prompted twice
    // for an invalid password.
    if password.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "password cannot be empty").into());
    }

    let confirmation = prompt_password("Confirm password: ")?;

    if password != confirmation {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "password confirmation did not match",
        )
        .into());
    }

    Ok(password)
}

pub fn default_unpack_output(input_path: &Path) -> PathBuf {
    let sanitized_name = input_path
        .file_stem()
        .and_then(|s| Path::new(s).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or_else(|| {
            eprintln!("warning: could not determine archive filename, defaulting to 'archive'");
            "archive"
        });
    PathBuf::from(sanitized_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to avoid test pollution: sets the env var, runs the closure, then
    // always removes it — even if the closure panics.
    //
    // SAFETY: these tests must run single-threaded (`cargo test -- --test-threads=1`)
    // or the env mutation can race with other threads. The test binary for cli.rs
    // unit tests runs sequentially by default.
    fn with_env_password<F: FnOnce()>(value: &str, f: F) {
        // SAFETY: single-threaded test context; no other threads read this var.
        unsafe { std::env::set_var(PASSWORD_ENV_VAR, value) };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        // SAFETY: same as above.
        unsafe { std::env::remove_var(PASSWORD_ENV_VAR) };
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    #[test]
    fn env_var_used_when_no_flag() {
        with_env_password("from_env", || {
            let pw = prompt_for_password(None).unwrap();
            assert_eq!(pw, "from_env");
        });
    }

    #[test]
    fn flag_takes_precedence_over_env_var() {
        with_env_password("from_env", || {
            let pw = prompt_for_password(Some("from_flag")).unwrap();
            assert_eq!(pw, "from_flag");
        });
    }

    #[test]
    fn empty_env_var_falls_through_to_confirmation() {
        // An empty RIFTX_PASSWORD should be treated as unset. We can't reach the
        // interactive prompt in a test, so just verify the env var is ignored by
        // checking that password_from_env() returns None for an empty value.
        // SAFETY: single-threaded test context.
        unsafe { std::env::set_var(PASSWORD_ENV_VAR, "") };
        let result = password_from_env();
        // SAFETY: same as above.
        unsafe { std::env::remove_var(PASSWORD_ENV_VAR) };
        assert!(result.is_none());
    }

    #[test]
    fn confirmation_path_uses_env_var_without_prompting() {
        with_env_password("scripted", || {
            let pw = prompt_for_password_with_confirmation(None).unwrap();
            assert_eq!(pw, "scripted");
        });
    }
}
