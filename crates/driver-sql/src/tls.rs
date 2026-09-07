use std::io::BufReader;

/// Backend TLS policy selected by the deployment, independently of Kubernetes authentication.
#[derive(Debug, Clone, Default)]
pub enum SqlTls {
    /// Validate the server certificate and logical hostname against public roots.
    #[default]
    Required,
    /// Validate using only these PEM certificate authority bytes, resolved by the runtime.
    WithCa(Vec<u8>),
    /// Explicit plaintext binding, for deployments whose database listener has no TLS.
    Disabled,
}

impl SqlTls {
    pub(crate) fn client_config(&self) -> Result<rustls::ClientConfig, crate::SqlDriverError> {
        let mut roots = rustls::RootCertStore::empty();
        match self {
            Self::Required => roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned()),
            Self::WithCa(bytes) => {
                for certificate in rustls_pemfile::certs(&mut BufReader::new(bytes.as_slice())) {
                    roots
                        .add(certificate.map_err(|_| invalid_ca())?)
                        .map_err(|_| invalid_ca())?;
                }
                if roots.is_empty() {
                    return Err(invalid_ca());
                }
            }
            Self::Disabled => return Err(invalid_ca()),
        }
        rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| invalid_ca())
        .map(|builder| builder.with_root_certificates(roots).with_no_client_auth())
    }
}

fn invalid_ca() -> crate::SqlDriverError {
    crate::SqlDriverError::Connection {
        detail: "database TLS configuration is invalid".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_ca_cannot_fall_back_to_public_roots_or_plaintext() {
        assert!(SqlTls::WithCa(Vec::new()).client_config().is_err());
        assert!(SqlTls::WithCa(b"not a certificate".to_vec())
            .client_config()
            .is_err());
        assert!(SqlTls::Required.client_config().is_ok());
    }
}
