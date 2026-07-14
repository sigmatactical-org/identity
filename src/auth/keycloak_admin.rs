mod approve_user_payload;
mod client_summary;
mod create_user_payload;
mod created_user;
mod keycloak_admin;
mod password_credential;
mod profile_input;
mod service_account_row;
mod token_response;
mod update_profile_payload;
mod update_user_payload;
mod user_profile;
mod user_representation;
mod user_summary;
pub(crate) use approve_user_payload::ApproveUserPayload;
pub(crate) use client_summary::ClientSummary;
pub(crate) use create_user_payload::CreateUserPayload;
pub(crate) use created_user::CreatedUser;
pub(crate) use keycloak_admin::KeycloakAdmin;
pub(crate) use password_credential::PasswordCredential;
pub(crate) use profile_input::ProfileInput;
pub(crate) use service_account_row::ServiceAccountRow;
pub(crate) use token_response::TokenResponse;
pub(crate) use update_profile_payload::UpdateProfilePayload;
pub(crate) use update_user_payload::UpdateUserPayload;
pub(crate) use user_profile::UserProfile;
pub(crate) use user_representation::UserRepresentation;
pub(crate) use user_summary::UserSummary;

use anyhow::{Context, Result};
use std::collections::HashMap;
use url::Url;

// Keycloak custom user-profile attribute keys. These must also be declared in
// the realm's user-profile config (see dev_realm.json) or Keycloak rejects
// writes to them.
const ATTR_PHONE: &str = "phone";
const ATTR_STREET: &str = "street";
const ATTR_CITY: &str = "city";
const ATTR_REGION: &str = "region";
const ATTR_POSTAL_CODE: &str = "postalCode";
const ATTR_COUNTRY: &str = "country";
const ATTR_BIRTHDATE: &str = "birthdate";
const ATTR_COMPANY: &str = "company";

/// First non-empty value for a Keycloak user attribute.
fn attribute_value(attributes: &Option<HashMap<String, Vec<String>>>, key: &str) -> Option<String> {
    attributes
        .as_ref()
        .and_then(|map| map.get(key))
        .and_then(|values| {
            values
                .iter()
                .find(|value| !value.trim().is_empty())
                .cloned()
        })
}

/// Set a single-valued attribute, or remove it when the value is blank.
fn set_or_remove_attribute(map: &mut HashMap<String, Vec<String>>, key: &str, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        map.remove(key);
    } else {
        map.insert(key.to_string(), vec![trimmed.to_string()]);
    }
}

fn user_id_from_location(location: &str) -> Result<String> {
    let user_id = location
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .context("Keycloak create user Location header has no user id")?;
    Ok(user_id.to_string())
}

fn parse_realm_issuer(issuer_url: &str) -> Result<(String, String)> {
    let issuer = Url::parse(issuer_url).context("Invalid OIDC issuer URL")?;
    let path = issuer.path().trim_end_matches('/');
    let realm = path
        .strip_prefix("/realms/")
        .filter(|name| !name.is_empty())
        .context("OIDC issuer URL must be of the form {scheme}://{host}/realms/{realm}")?;
    let server_base = issuer
        .join("/")
        .context("Invalid OIDC issuer origin")?
        .to_string()
        .trim_end_matches('/')
        .to_string();
    Ok((server_base, realm.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_issuer_extracts_realm_and_server() {
        let (base, realm) =
            parse_realm_issuer("http://127.0.0.1:8101/realms/multcorp").expect("parse issuer");
        assert_eq!(base, "http://127.0.0.1:8101");
        assert_eq!(realm, "multcorp");
    }

    #[test]
    fn location_header_yields_user_id() {
        assert_eq!(
            user_id_from_location("http://keycloak/admin/realms/multcorp/users/abc-123-def")
                .expect("user id"),
            "abc-123-def"
        );
    }
}
