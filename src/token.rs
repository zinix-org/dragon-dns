/*
 * Copyright 2026-present Viktor Popp
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use reqwest::header::AUTHORIZATION;
use serde::Deserialize;

pub enum TokenStatus {
    Active,
    Disabled,
    Expired,
    Missing,
}

#[derive(Debug, Deserialize)]
pub struct TokenVerificationResult {
    status: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenVerificationResponse {
    result: Option<TokenVerificationResult>,
    success: bool,
}

pub fn get_token_status(token: &str) -> TokenStatus {
    let client = reqwest::blocking::Client::new();
    let res_raw = client
        .get("https://api.cloudflare.com/client/v4/user/tokens/verify")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .expect("failed to send token verification request")
        .text()
        .expect("failed to get token verification text");
    let res: TokenVerificationResponse =
        serde_json::from_str(&res_raw).expect("failed to parse JSON");

    if res.result.is_none() {
        return TokenStatus::Missing;
    }

    match res.result.unwrap().status.as_str() {
        "active" => TokenStatus::Active,
        "disabled" => TokenStatus::Disabled,
        "expired" => TokenStatus::Expired,
        _ => TokenStatus::Missing,
    }
}
