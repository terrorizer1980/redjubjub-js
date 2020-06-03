#![cfg_attr(feature = "nightly", feature(external_doc))]
#![cfg_attr(feature = "nightly", doc(include = "../README.md"))]

//! Docs require the `nightly` feature until RFC 1990 lands.
extern crate hex;
extern crate rand;
extern crate wasm_bindgen;

use std::convert::TryFrom;

use rand::thread_rng;
use wasm_bindgen::prelude::*;

pub use error::Error;
use hash::HStar;
pub use public_key::{PublicKey, PublicKeyBytes};
pub use secret_key::SecretKey;
pub use signature::Signature;

mod constants;
mod error;
mod hash;
mod public_key;
mod secret_key;
mod signature;

/// An element of the JubJub scalar field used for randomization of public and secret keys.
pub type Randomizer = jubjub::Fr;

/// A better name than Fr.
// XXX-jubjub: upstream this name
type Scalar = jubjub::Fr;

/// Abstracts over different RedJubJub parameter choices, [`Binding`]
/// and [`SpendAuth`].
///
/// As described [at the end of §5.4.6][concretereddsa] of the Zcash
/// protocol specification, the generator used in RedJubjub is left as
/// an unspecified parameter, chosen differently for each of
/// `BindingSig` and `SpendAuthSig`.
///
/// To handle this, we encode the parameter choice as a genuine type
/// parameter.
///
/// [concretereddsa]: https://zips.z.cash/protocol/protocol.pdf#concretereddsa
pub trait SigType: private::Sealed {}

/// A type variable corresponding to Zcash's `BindingSig`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Binding {}

impl SigType for Binding {}

/// A type variable corresponding to Zcash's `SpendAuthSig`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SpendAuth {}

impl SigType for SpendAuth {}

pub(crate) mod private {
    use super::*;

    pub trait Sealed: Copy + Clone + Eq + PartialEq + std::fmt::Debug {
        fn basepoint() -> jubjub::ExtendedPoint;
    }

    impl Sealed for Binding {
        fn basepoint() -> jubjub::ExtendedPoint {
            jubjub::AffinePoint::from_bytes(constants::BINDINGSIG_BASEPOINT_BYTES)
                .unwrap()
                .into()
        }
    }

    impl Sealed for SpendAuth {
        fn basepoint() -> jubjub::ExtendedPoint {
            jubjub::AffinePoint::from_bytes(constants::SPENDAUTHSIG_BASEPOINT_BYTES)
                .unwrap()
                .into()
        }
    }
}

fn ask_to_rk(ask_string: String, alpha_string: String) -> Result<SecretKey<SpendAuth>, Error> {
    let mut alpha_bytes = [0u8;32];
    hex::decode_to_slice(alpha_string, &mut alpha_bytes).expect("Parse error!");
    let maybe_alpha = Scalar::from_bytes(&alpha_bytes);
    let alpha_scalar = {
        if maybe_alpha.is_some().into() {
            maybe_alpha.unwrap()
        } else {
            return Err(Error::MalformedSecretKey);
        }
    };

    let mut ask_bytes = [0u8;32];
    hex::decode_to_slice(ask_string, &mut ask_bytes).expect("Parse error!");
    let sk = SecretKey::<SpendAuth>::try_from(ask_bytes);
    if sk.is_ok() {
        Ok(sk.unwrap().randomize(&alpha_scalar))
    } else {
        Err(Error::MalformedSecretKey)
    }
}

//@TODO delete later
// rsk --> rk
#[wasm_bindgen]
pub fn generate_rk_from_ask(ask_string: String, alpha_string: String) -> String {

    let sk = match ask_to_rk(ask_string, alpha_string) {
        Ok(p) => p,
        Err(_) => return String::new(),
    };

    hex::encode(sk.pk.bytes.bytes)
}

#[test]
fn rk_test() {
    let ask = String::from("0100000000000000000000000000000000000000000000000000000000000000");
    let alpha = String::from("0100000000000000000000000000000000000000000000000000000000000000");
    let rk = generate_rk_from_ask(ask, alpha);

    println!("rk {}", rk);
}

// sign msg with sk
#[wasm_bindgen]
pub fn generate_spend_auth_sig(ask_string: String, alpha_string: String, message_hash_string: String) -> String {
    let sk = match ask_to_rk(ask_string, alpha_string) {
        Ok(p) => p,
        Err(_) => return String::new(),
    };

    let mut message_hash = [0u8;32];
    match hex::decode_to_slice(message_hash_string, &mut message_hash) {
        Ok(()) => (),
        Err(_) => return String::new(),
    };
    let sig = sk.sign(thread_rng(), message_hash.as_ref());

// Types can be converted to raw byte arrays using From/Into
    let r_str = hex::encode(sig.r_bytes);
    let s_str = hex::encode(sig.s_bytes);

    r_str + &s_str
}

#[test]
fn auth_test() {
    let msg = String::from("0102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f00");
    let ask = String::from("0100000000000000000000000000000000000000000000000000000000000000");
    let alpha = String::from("0100000000000000000000000000000000000000000000000000000000000000");

    let sig = generate_spend_auth_sig(ask, alpha, msg);
    println!("sig {}", sig);
}

#[wasm_bindgen]
pub fn verify_spend_auth_sig(rk_string: String, message_hash_string: String, signature_string: String) -> bool {
    let mut message_hash = [0u8;32];
    match hex::decode_to_slice(message_hash_string, &mut message_hash) {
        Ok(()) => (),
        Err(_) => return false,
    };

    let mut rk_bytes = [0u8;32];
    match hex::decode_to_slice(rk_string, &mut rk_bytes) {
        Ok(()) => (),
        Err(_) => return false,
    };
    let pk = match PublicKey::<SpendAuth>::try_from(rk_bytes) {
        Ok(p) => p,
        Err(_) => return false,
    };

    let mut signature_bytes = [0u8;64];
    match hex::decode_to_slice(signature_string, &mut signature_bytes) {
        Ok(()) => (),
        Err(_) => return false,
    };
    let sig = Signature::<SpendAuth>::from(signature_bytes);

    match pk.verify(message_hash.as_ref(), &sig) {
        Ok(()) => true,
        Err(_) => false,
    }
}

#[test]
fn verify_auth_test() {
    let sig = String::from("d8b672c77d91ffa12c1224e1121be707e2de75d2132f9d6833d491a63720531b0bda7a79e8ec2f8cee7c98f165c5c4c654d8973721bd8a70defc3cd973ae6106");
    let msg = String::from("0102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f00");
    let rk = String::from("b14361aaf420d30d3e8bcc7c5c34f5025abc86abb2aafcc35831749ea62e9c5d");

    assert!(verify_spend_auth_sig(rk, msg, sig));
}

#[wasm_bindgen]
pub fn generate_pk_from_sk(sk_string: String) -> String {
    let mut sk_bytes = [0u8;32];
    match hex::decode_to_slice(sk_string, &mut sk_bytes) {
        Ok(()) => (),
        Err(_) => return String::new(),
    };
    let sk = match SecretKey::<Binding>::try_from(sk_bytes) {
        Ok(p) => p,
        Err(_) => return String::new(),
    };

    hex::encode(sk.pk.bytes.bytes)
}

#[test]
fn pk_test() {
    let sk = String::from("0100000000000000000000000000000000000000000000000000000000000000");
    let pk = generate_pk_from_sk(sk);

    println!("pk {}", pk);
}

#[wasm_bindgen]
pub fn generate_binding_sig(sk_string: String, message_hash_string: String) -> String {
    let mut sk_bytes = [0u8;32];
    match hex::decode_to_slice(sk_string, &mut sk_bytes) {
        Ok(()) => (),
        Err(_) => return String::new(),
    };
    let sk = match SecretKey::<Binding>::try_from(sk_bytes) {
        Ok(p) => p,
        Err(_) => return String::new(),
    };

    let mut message_hash = [0u8;32];
    match hex::decode_to_slice(message_hash_string, &mut message_hash) {
        Ok(()) => (),
        Err(_) => return String::new(),
    };

    let sig = sk.sign(thread_rng(), message_hash.as_ref());
    let r_str = hex::encode(sig.r_bytes);
    let s_str = hex::encode(sig.s_bytes);

    r_str + &s_str
}

#[test]
fn binding_sig_test() {
    let msg = String::from("0102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f00");
    let sk = String::from("0100000000000000000000000000000000000000000000000000000000000000");

    let sig = generate_binding_sig(sk, msg);
    println!("sig {}", sig);
}

#[wasm_bindgen]
pub fn verify_binding_sig(pk_string: String, message_hash_string: String, signature_string: String) -> bool {
    let mut pk_bytes = [0u8;32];
    match hex::decode_to_slice(pk_string, &mut pk_bytes) {
        Ok(()) => (),
        Err(_) => return false,
    };
    let public_key = match PublicKey::<Binding>::try_from(pk_bytes) {
        Ok(p) => p,
        Err(_) => return false,
    };

    let mut signature_bytes = [0u8;64];
    match hex::decode_to_slice(signature_string, &mut signature_bytes) {
        Ok(()) => (),
        Err(_) => return false,
    };
    let sig = Signature::<Binding>::from(signature_bytes);

    let mut message_hash = [0u8;32];
    match hex::decode_to_slice(message_hash_string, &mut message_hash) {
        Ok(()) => (),
        Err(_) => return false,
    };

    match public_key.verify(message_hash.as_ref(), &sig) {
        Ok(()) => true,
        Err(_) => false,
    }
}

#[test]
fn verify_binding_test() {
    let sig = String::from("26fc82e2c90c322107fbb22e542097e0eabfe7ec006722583a1289f7e80ea0c633c606b026d9f73803bf5a2b744457a10b2709c5a6c6a257bf54db557ebc590c");
    let msg = String::from("0102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f00");
    let pk = String::from("8b6a0b38b9faae3c3b803b47b0f146ad50ab221e6e2afbe6dbde45cba9d381ed");

    assert!(verify_binding_sig(pk, msg, sig));
}
