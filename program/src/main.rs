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

use ark_ec::{pairing::Pairing, VariableBaseMSM};
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
    let bigints: Vec<_> = scalars.0.into_iter().map(|s| s.into()).collect();
    let result = <E as Pairing>::G1::msm_bigint(&points.0, &bigints);

    let mut bytes = vec![];
    result.serialize_uncompressed(&mut bytes).unwrap();
    sp1_zkvm::io::commit_slice(&bytes);
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
