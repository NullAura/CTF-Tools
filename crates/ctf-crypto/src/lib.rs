//! Crypto helpers.

use ctf_core::{
    OperationOutput, OperationRequest, OperationResponse, OperationRunner, OperationSpec, Result,
};
use md4::Md4;
use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use sha3::Sha3_256;
use sm3::Sm3;

pub fn register_handlers(runner: &mut OperationRunner) {
    runner.register_handler("hash.md5", hash_md5);
    runner.register_handler("hash.sha1", hash_sha1);
    runner.register_handler("hash.sha256", hash_sha256);
    runner.register_handler("hash.sha512", hash_sha512);
    runner.register_handler("hash.sha3_256", hash_sha3_256);
    runner.register_handler("hash.ntlm", hash_ntlm);
    runner.register_handler("hash.sm3", hash_sm3);
}

fn hash_md5(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    hash::<Md5>("md5", &request.input_bytes()?)
}

fn hash_sha1(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    hash::<Sha1>("sha1", &request.input_bytes()?)
}

fn hash_sha256(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    hash::<Sha256>("sha256", &request.input_bytes()?)
}

fn hash_sha512(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    hash::<Sha512>("sha512", &request.input_bytes()?)
}

fn hash_sha3_256(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    hash::<Sha3_256>("sha3-256", &request.input_bytes()?)
}

fn hash_sm3(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    hash::<Sm3>("sm3", &request.input_bytes()?)
}

fn hash_ntlm(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let mut bytes = Vec::new();
    for code in request.input_text()?.encode_utf16() {
        bytes.extend_from_slice(&code.to_le_bytes());
    }
    hash::<Md4>("ntlm", &bytes)
}

fn hash<D>(label: &str, bytes: &[u8]) -> Result<OperationResponse>
where
    D: Digest + Default,
{
    let digest = D::digest(bytes);
    Ok(OperationResponse {
        status: "ok".to_string(),
        outputs: vec![OperationOutput {
            kind: "text".to_string(),
            label: label.to_string(),
            value: hex::encode(digest),
        }],
        warnings: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(operation: &str, value: &str) -> OperationRequest {
        OperationRequest {
            operation: operation.to_string(),
            input: ctf_core::OperationInput {
                kind: "text".to_string(),
                value: value.to_string(),
            },
            limits: ctf_core::TaskLimits::default(),
        }
    }

    #[test]
    fn md5_hashes_text() {
        let response = hash_md5(&dummy_spec(), &request("hash.md5", "abc")).expect("md5");
        assert_eq!(
            response.outputs[0].value,
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }

    #[test]
    fn ntlm_hashes_password() {
        let response = hash_ntlm(&dummy_spec(), &request("hash.ntlm", "password")).expect("ntlm");
        assert_eq!(
            response.outputs[0].value,
            "8846f7eaee8fb117ad06bdd830b7586c"
        );
    }

    fn dummy_spec() -> OperationSpec {
        OperationSpec {
            id: "test".to_string(),
            name_zh: "test".to_string(),
            name_en: "test".to_string(),
            category: "test".to_string(),
            aliases: vec![],
            input: vec!["text".to_string()],
            output: vec!["text".to_string()],
            backend: "rust".to_string(),
            safety: "safe".to_string(),
            deterministic: true,
            batchable: true,
            priority: "P0".to_string(),
            secrets: None,
        }
    }
}
