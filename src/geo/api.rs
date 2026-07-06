//! Public HTTP handler for country region lookups.

use axum::{Json, extract::Query};
use serde::{Deserialize, Serialize};

use super::{country_code_for_name, region_prompt_for_country_code, regions_for_country_code};

#[derive(Debug, Deserialize)]
pub struct RegionsQuery {
    pub country: String,
}

#[derive(Debug, Serialize)]
pub struct RegionsResponse {
    pub prompt: String,
    pub regions: Vec<&'static str>,
}

/// `GET /geo/regions?country=…` — returns first-level regions for a country name.
pub async fn regions_handler(Query(query): Query<RegionsQuery>) -> Json<RegionsResponse> {
    Json(regions_for_country(&query.country))
}

#[must_use]
pub fn regions_for_country(country: &str) -> RegionsResponse {
    let Some(code) = country_code_for_name(country) else {
        return RegionsResponse {
            prompt: String::new(),
            regions: Vec::new(),
        };
    };
    let Some(regions) = regions_for_country_code(code) else {
        return RegionsResponse {
            prompt: String::new(),
            regions: Vec::new(),
        };
    };
    RegionsResponse {
        prompt: region_prompt_for_country_code(code).to_string(),
        regions: regions.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn united_states_returns_states() {
        let body = regions_for_country("United States");
        assert_eq!(body.prompt, "Select a state…");
        assert!(body.regions.contains(&"California"));
    }

    #[test]
    fn mexico_returns_states() {
        let body = regions_for_country("Mexico");
        assert_eq!(body.prompt, "Select a province…");
        assert!(body.regions.contains(&"Jalisco"));
    }

    #[test]
    fn unknown_country_returns_empty() {
        let body = regions_for_country("Not A Country");
        assert!(body.regions.is_empty());
        assert!(body.prompt.is_empty());
    }
}
