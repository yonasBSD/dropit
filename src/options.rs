use std::net::IpAddr;
use std::path::PathBuf;

use byte_unit::{Byte, ByteError};
use clap::AppSettings::DeriveDisplayOrder;
use clap::ArgGroup;
use clap::Parser;
use log::LevelFilter;

use crate::auth::{Credential, Features, LdapAuthProcess, LdapAuthenticator, Origin};
use crate::upload::Threshold;

#[derive(Parser, Debug)]
#[clap(version, about, setting = DeriveDisplayOrder)]
#[clap(
    group(ArgGroup::new("origin").required(true).args(&["ip-origin", "username-origin"])),
    group(ArgGroup::new("auth").multiple(true).args(&["credentials", "ldap-address"])),
    group(ArgGroup::new("ldap-process").args(&["ldap-dn-pattern", "ldap-search-base-dn"])),
)]
pub struct Options {
    /// Increase logs verbosity (Error (default), Warn, Info, Debug, Trace).
    #[clap(short = 'v', long = "verbose", parse(from_occurrences = parse_log_level))]
    pub log_level: LevelFilter,
    /// Upload files directory path (relative).
    #[clap(short = 'u', long, default_value = "uploads")]
    pub uploads_dir: PathBuf,
    /// Disable upload files directory automatic creation (if missing).
    #[clap(short = 'U', long)]
    pub no_uploads_dir_creation: bool,
    /// Metadata database path (relative).
    #[clap(short = 'd', long, default_value = "dropit.db")]
    pub database: PathBuf,
    /// Disable metadata database automatic creation (if missing).
    #[clap(short = 'D', long)]
    pub no_database_creation: bool,
    /// HTTP listening address.
    #[clap(short = 'a', long, default_value = "127.0.0.1")]
    pub address: IpAddr,
    /// HTTP listening port.
    #[clap(short = 'p', long, default_value = "8080")]
    pub port: u16,
    /// Use X-Forwarded-For, X-Forwarded-Proto and X-Forwarded-Host to determine uploads' origin.
    #[clap(short = 'R', long = "behind-reverse-proxy")]
    pub behind_proxy: bool,
    /// Relations between files' sizes and their durations. Must be ordered by increasing size and decreasing duration.
    #[clap(short = 't', long = "threshold", required = true)]
    pub thresholds: Vec<Threshold>,
    /// Use usernames as uploaders' identities.
    #[clap(short = 'o', long)]
    pub ip_origin: bool,
    /// Use IP addresses as uploaders' identities.
    #[clap(short = 'O', long, requires = "auth")]
    pub username_origin: bool,
    /// Cumulative size limit from the same uploader.
    #[clap(short = 's', long, required = true, parse(try_from_str = parse_size))]
    pub origin_size_sum: u64,
    /// Number of files limit from the same uploader.
    #[clap(short = 'c', long, required = true)]
    pub origin_file_count: usize,
    /// Cumulative size limit from all users.
    #[clap(short = 'S', long, required = true, parse(try_from_str = parse_size))]
    pub global_size_sum: u64,
    /// Protect upload endpoint with authentication.
    #[clap(long, requires = "auth")]
    pub auth_upload: bool,
    /// Protect download endpoint with authentication.
    #[clap(long, requires = "auth")]
    pub auth_download: bool,
    /// Static list of credentials.
    #[clap(short = 'C', long = "credential")]
    pub credentials: Vec<Credential>,
    /// URI of the LDAP used to authenticate users.
    #[clap(long, requires = "ldap-process")]
    pub ldap_address: Option<String>,
    /// LDAP DN pattern used when using single bind process.
    #[clap(long, requires = "ldap-address")]
    pub ldap_dn_pattern: Option<String>,
    /// LDAP base DN used during username searches.
    #[clap(long, requires = "ldap-address")]
    pub ldap_search_base_dn: Option<String>,
    /// LDAP attribute(s) pattern used to match usernames during searches.
    #[clap(long, default_value = "(uid=%u)", requires = "ldap-search-base-dn")]
    pub ldap_search_attribute_pattern: String,
    /// LDAP DN used to bind during username searches.
    #[clap(long, requires_all = &["ldap-search-base-dn", "ldap-search-password"])]
    pub ldap_search_dn: Option<String>,
    /// LDAP password used to bind during username searches.
    #[clap(long, requires = "ldap-search-dn")]
    pub ldap_search_password: Option<String>,
    /// CSS color used in the web UI.
    #[clap(short = 'T', long, default_value = "#15b154")]
    pub theme: String,
}

impl Options {
    pub fn origin(&self) -> Option<Origin> {
        if self.ip_origin {
            Some(Origin::IpAddress)
        } else if self.username_origin {
            Some(Origin::Username)
        } else {
            None
        }
    }

    pub fn access(&self) -> Features {
        let mut access = Features::empty();
        if self.auth_upload {
            access.insert(Features::UPLOAD);
        }
        if self.auth_download {
            access.insert(Features::DOWNLOAD);
        }
        access
    }

    pub fn ldap_authenticator(&self) -> Option<LdapAuthenticator> {
        let process = match (&self.ldap_dn_pattern, &self.ldap_search_base_dn) {
            (Some(dn_pattern), _) => LdapAuthProcess::SingleBind {
                dn_pattern: dn_pattern.clone(),
            },
            (_, Some(base_dn)) => LdapAuthProcess::SearchThenBind {
                search_credentials: self.ldap_search_dn.as_ref().and_then(|lsd| {
                    self.ldap_search_password
                        .as_ref()
                        .map(|lsp| (lsd.clone(), lsp.clone()))
                }),
                base_dn: base_dn.clone(),
                filter_pattern: self.ldap_search_attribute_pattern.clone(),
            },
            _ => return None,
        };
        Some(LdapAuthenticator::new(
            self.ldap_address.as_ref()?.clone(),
            process,
        ))
    }
}

fn parse_size(s: &str) -> Result<u64, ByteError> {
    Ok(s.parse::<Byte>()?.get_bytes())
}

fn parse_log_level(n: u64) -> LevelFilter {
    use LevelFilter::*;
    match n {
        0 => Error,
        1 => Warn,
        2 => Info,
        3 => Debug,
        _ => Trace,
    }
}

#[cfg(test)]
mod tests {
    use super::Options;
    use clap::error::{ContextKind, ContextValue};
    use clap::{Error, ErrorKind, Parser};
    use itertools::Itertools;

    macro_rules! cmd {
        ($($arg:tt)*) => {
            {
                Options::try_parse_from([
                    "dropit",
                    "--threshold",
                    "100kb:5m",
                    "--origin-size-sum",
                    "1mb",
                    "--origin-file-count",
                    "1",
                    "--global-size-sum",
                    "10mb",
                    $($arg)*
                ])
            }
        }
    }

    fn missing_args<const N: usize>(err: Error, names: [&str; N]) {
        assert!(
            err.kind() == ErrorKind::MissingRequiredArgument
                && names.into_iter().all(|name| err
                    .context()
                    .find(|(k, v)| {
                        matches!(k, ContextKind::InvalidArg)
                            && match v {
                                ContextValue::Strings(ss) => ss.iter().any(|s| s.contains(name)),
                                _ => false,
                            }
                    })
                    .is_some())
        )
    }

    fn conflict(err: Error, rhs: &str, lhs: &str) {
        assert!(
            err.kind() == ErrorKind::ArgumentConflict && {
                let (a1, a2) = err
                    .context()
                    .filter(|(k, _)| matches!(k, ContextKind::InvalidArg | ContextKind::PriorArg))
                    .filter_map(|(_, v)| match v {
                        ContextValue::String(s) => Some(s),
                        _ => None,
                    })
                    .collect_tuple()
                    .unwrap();
                a1.contains(rhs) && a2.contains(lhs) || a1.contains(lhs) && a2.contains(rhs)
            }
        )
    }

    #[test]
    fn basic() {
        // Missing all base options.
        missing_args(
            Options::try_parse_from(["dropit"]).unwrap_err(),
            [
                "threshold",
                "origin-size-sum",
                "origin-file-count",
                "global-size-sum",
            ],
        );

        // All base options provided.
        assert!(cmd!["--ip-origin"].is_ok());
    }

    #[test]
    fn origin() {
        // Missing origin.
        missing_args(cmd![].unwrap_err(), ["ip-origin", "username-origin"]);

        // Duplicated origins.
        conflict(
            cmd!["--ip-origin", "--username-origin"].unwrap_err(),
            "ip-origin",
            "username-origin",
        );

        // Missing auth method while using username origin.
        missing_args(
            cmd!["--username-origin"].unwrap_err(),
            ["credential", "ldap-address"],
        );

        // Username origin with static credentials.
        assert!(cmd!["--username-origin", "--credential", "username:password"].is_ok());

        // Username origin with LDAP.
        assert!(cmd![
            "--username-origin",
            "--ldap-address",
            "ldap://10.0.0.1",
            "--ldap-search-base-dn",
            "ou=Identities,dc=myOrg",
        ]
        .is_ok());
    }

    #[test]
    fn auth() {
        // Protect upload and missing auth method.
        missing_args(
            cmd!["--auth-upload"].unwrap_err(),
            ["credential", "ldap-address"],
        );

        // Protect download and missing auth method.
        missing_args(
            cmd!["--auth-download"].unwrap_err(),
            ["credential", "ldap-address"],
        );

        // Protect with static credentials.
        assert!(cmd![
            "--ip-origin",
            "--auth-upload",
            "--credential",
            "username:password",
        ]
        .is_ok());

        // Protect with LDAP.
        assert!(cmd![
            "--ip-origin",
            "--auth-upload",
            "--ldap-address",
            "ldap://10.0.0.1",
            "--ldap-dn-pattern",
            "org=MyOrg,uid=%u"
        ]
        .is_ok());

        // Both static credentials and LDAP.
        assert!(cmd![
            "--ip-origin",
            "--auth-upload",
            "--credential",
            "username:password",
            "--ldap-address",
            "ldap://10.0.0.1",
            "--ldap-dn-pattern",
            "org=MyOrg,uid=%u"
        ]
        .is_ok());
    }

    #[test]
    fn ldap() {
        // LDAP with missing auth process.
        missing_args(
            cmd!["--ldap-address", "ldap://10.0.0.1"].unwrap_err(),
            ["ldap-dn-pattern", "ldap-search-base-dn"],
        );

        // LDAP with direct bind.
        assert!(cmd![
            "--ip-origin",
            "--ldap-address",
            "ldap://10.0.0.1",
            "--ldap-dn-pattern",
            "org=MyOrg,uid=%u"
        ]
        .is_ok());

        // LDAP with search process.
        assert!(cmd![
            "--ip-origin",
            "--ldap-address",
            "ldap://10.0.0.1",
            "--ldap-search-base-dn",
            "ou=Identities,dc=myOrg",
        ]
        .is_ok());

        // LDAP with search process and missing username.
        missing_args(
            cmd![
                "--ldap-address",
                "ldap://10.0.0.1",
                "--ldap-search-base-dn",
                "ou=Identities,dc=myOrg",
                "--ldap-search-password",
                "password1234",
            ]
            .unwrap_err(),
            ["ldap-search-dn"],
        );

        // LDAP with search process and missing password.
        missing_args(
            cmd![
                "--ldap-address",
                "ldap://10.0.0.1",
                "--ldap-search-base-dn",
                "ou=Identities,dc=myOrg",
                "--ldap-search-dn",
                "uid=user1234",
            ]
            .unwrap_err(),
            ["ldap-search-password"],
        );

        // LDAP with search process, specified attributes but missing search base dn.
        missing_args(
            cmd![
                "--ldap-address",
                "ldap://10.0.0.1",
                "--ldap-search-attribute-pattern",
                "(email=%u)",
            ]
            .unwrap_err(),
            ["ldap-search-base-dn"],
        );

        // LDAP with both process.
        conflict(
            cmd![
                "--ldap-address",
                "ldap://10.0.0.1",
                "--ldap-dn-pattern",
                "org=MyOrg,uid=%u",
                "--ldap-search-base-dn",
                "ou=Identities,dc=myOrg",
            ]
            .unwrap_err(),
            "ldap-dn-pattern",
            "ldap-search-base-dn",
        )
    }
}
