

//// Topics:
//// authentication, authorization, JWT decoding 
////
//// - Decoding the JWT - authentication
//// - Checking `features.contains ("new_allocation")` - authorization which 
////   is in tw_main.rs :: fn buy() service handler, can be here as well.


use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Claims {
    sub: String,
    pub features: Vec<String>,
}

/// Used for pre-API versioning workaround purposes 
pub fn decode_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.required_spec_claims.remove("exp");
        // remove mandatory "exp" for debugging
    let data = decode::<Claims>(
        token, &DecodingKey::from_secret(b"secret"), &validation
    )?;
    Ok(data.claims)
}