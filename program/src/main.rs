//! This program proves that the executed transactions are correctly derived from an espresso block.
// Inputs:
//  - Namespace ID (public)
//  - Namespace table (public)
//  - VID commitment (public)
//  - Rollup transactions commitment (public)
//  - An index in the namespace table for the rollup
//  - Two offsets that defines the namespace range
//  - All transactions
// This program proves that
//  - The namespace table contains an entry of this namespace ID.
//  - Transactions given by two offsets in the (VID) committed payload are ones committed by rollup.

#![no_main]

use ark_ec::pairing::Pairing;
use ark_ff::{BigInt, BigInteger, PrimeField, Zero};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde::{de::Error as _, ser::Error as _, Deserialize, Serialize};

sp1_zkvm::entrypoint!(main);

pub fn main() {
    let points = sp1_zkvm::io::read::<VecG1Affine>();
    let scalars = sp1_zkvm::io::read::<VecScalarField>();

    // let result: ark_bn254::G1Projective = points
    //     .0
    //     .into_iter()
    //     .zip(scalars.0)
    //     .map(|(p, s)| p * s)
    //     .sum();
    // let bigints: Vec<_> = scalars.0.into_iter().map(|s| s.into()).collect();
    // let result = <E as Pairing>::G1::msm_bigint(&points.0, &bigints);
    // let result = unwrap_g1affine(&result);
    std::println!("cycle-tracker-start: msm");
    let result = msm(&points.0, &scalars.0);
    std::println!("cycle-tracker-end: msm");
    std::println!("{}", result);

    let mut bytes = vec![];
    result.serialize_uncompressed(&mut bytes).unwrap();
    sp1_zkvm::io::commit_slice(&bytes);
}

#[derive(Clone, Copy, Debug)]
pub struct BnAffinePoint(pub BigInt<4>, pub BigInt<4>);

fn wrap_g1affine(p: &G1Affine) -> BnAffinePoint {
    BnAffinePoint(p.x.into_bigint(), p.y.into_bigint())
}

fn unwrap_g1affine(p: &BnAffinePoint) -> G1Affine {
    G1Affine {
        x: p.0.into(),
        y: p.1.into(),
        infinity: false,
    }
}

fn msm(p: &[G1Affine], s: &[ScalarField]) -> G1Affine {
    let mut iter = p.iter().zip(s).filter(|(_, s)| !s.is_zero());
    let mut result = {
        let (p, s) = iter.next().unwrap();
        let mut p = wrap_g1affine(p);
        bn254_double_and_add(&mut p, s);
        p
    };
    iter.for_each(|(p, s)| {
        let mut p = wrap_g1affine(p);
        bn254_double_and_add(&mut p, s);
        bn254_add(&mut result, &p);
    });
    unwrap_g1affine(&result)
}

fn bn254_add(p: &mut BnAffinePoint, q: &BnAffinePoint) {
    sp1_zkvm::syscalls::syscall_bn254_add(
        p as *mut _ as *mut [u32; 16],
        q as *const _ as *const [u32; 16],
    );
}

fn bn254_double_and_add(p: &mut BnAffinePoint, s: &ScalarField) {
    let mut t = *p;
    let mut b = s.into_bigint().to_bits_le().into_iter();
    let mut q = {
        b.by_ref().take_while(|b| !b).for_each(|_| {
            sp1_zkvm::syscalls::syscall_bn254_double(&mut t as *mut _ as *mut [u32; 16]);
        });
        t
    };
    b.for_each(|b| {
        // double in place
        sp1_zkvm::syscalls::syscall_bn254_double(&mut t as *mut _ as *mut [u32; 16]);
        if b {
            sp1_zkvm::syscalls::syscall_bn254_add(
                &mut q as *mut _ as *mut [u32; 16],
                &t as *const _ as *const [u32; 16],
            );
        }
    });

    *p = q
}

type E = ark_bn254::Bn254;
type G1Affine = <E as Pairing>::G1Affine;

#[derive(Debug, CanonicalSerialize, CanonicalDeserialize)]
struct VecG1Affine(pub Vec<G1Affine>);

impl Serialize for VecG1Affine {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut bytes = Vec::new();
        self.0
            .serialize_uncompressed(&mut bytes)
            .map_err(|e| S::Error::custom(format!("{e:?}")))?;
        Serialize::serialize(&bytes, serializer)
    }
}

impl<'de> Deserialize<'de> for VecG1Affine {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = <Vec<u8> as Deserialize>::deserialize(deserializer)?;
        <Self as CanonicalDeserialize>::deserialize_uncompressed_unchecked(&*bytes)
            .map_err(|e| D::Error::custom(format!("{e:?}")))
    }
}

type ScalarField = <E as Pairing>::ScalarField;

#[derive(Debug, CanonicalSerialize, CanonicalDeserialize)]
struct VecScalarField(pub Vec<ScalarField>);

impl Serialize for VecScalarField {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut bytes = Vec::new();
        self.0
            .serialize_uncompressed(&mut bytes)
            .map_err(|e| S::Error::custom(format!("{e:?}")))?;
        Serialize::serialize(&bytes, serializer)
    }
}

impl<'de> Deserialize<'de> for VecScalarField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = <Vec<u8> as Deserialize>::deserialize(deserializer)?;
        <Self as CanonicalDeserialize>::deserialize_uncompressed_unchecked(&*bytes)
            .map_err(|e| D::Error::custom(format!("{e:?}")))
    }
}
