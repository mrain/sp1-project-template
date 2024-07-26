//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be verified
//! on-chain.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run --package fibonacci-script --bin prove --release
//! ```

use std::path::PathBuf;

use alloy_sol_types::{sol, SolType};
use ark_ec::pairing::Pairing;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::UniformRand;
use clap::Parser;
use serde::{de::Error as _, ser::Error as _, Deserialize, Serialize};
use sp1_sdk::{HashableKey, ProverClient, SP1PlonkBn254Proof, SP1Stdin, SP1VerifyingKey};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
///
/// This file is generated by running `cargo prove build` inside the `program` directory.
pub const FIBONACCI_ELF: &[u8] = include_bytes!("../../../program/elf/riscv32im-succinct-zkvm-elf");

/// The arguments for the prove command.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct ProveArgs {
    #[clap(long, default_value = "20")]
    n: u32,

    #[clap(long, default_value = "false")]
    evm: bool,
}

/// The public values encoded as a tuple that can be easily deserialized inside Solidity.
type PublicValuesTuple = sol! {
    tuple(uint32, uint32, uint32)
};

fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();

    // Parse the command line arguments.
    let args = ProveArgs::parse();

    // Setup the prover client.
    let client = ProverClient::new();

    // Setup the program.
    let (pk, vk) = client.setup(FIBONACCI_ELF);

    // Setup the inputs.;
    let mut stdin = SP1Stdin::new();
    println!("n: {}", args.n);
    let mut rng = jf_utils::test_rng();
    let points = VecG1Affine((0..args.n).map(|_| G1Affine::rand(&mut rng)).collect());
    stdin.write(&points);
    let scalars = VecScalarField((0..args.n).map(|_| ScalarField::rand(&mut rng)).collect());
    stdin.write(&scalars);

    if args.evm {
        // Generate the proof.
        let proof = client
            .prove_plonk(&pk, stdin)
            .expect("failed to generate proof");
        create_plonk_fixture(&proof, &vk);
    } else {
        // Generate the proof.
        let proof = client.prove(&pk, stdin).expect("failed to generate proof");
        let result = <ark_bn254::G1Projective as CanonicalDeserialize>::deserialize_uncompressed(
            proof.public_values.as_slice(),
        )
        .unwrap();
        println!("Successfully generated proof!");
        println!("result: {}", result);

        // Verify the proof.
        client.verify(&proof, &vk).expect("failed to verify proof");
    }
}

/// A fixture that can be used to test the verification of SP1 zkVM proofs inside Solidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SP1FibonacciProofFixture {
    a: u32,
    b: u32,
    n: u32,
    vkey: String,
    public_values: String,
    proof: String,
}

/// Create a fixture for the given proof.
fn create_plonk_fixture(proof: &SP1PlonkBn254Proof, vk: &SP1VerifyingKey) {
    // Deserialize the public values.
    let bytes = proof.public_values.as_slice();
    let (n, a, b) = PublicValuesTuple::abi_decode(bytes, false).unwrap();

    // Create the testing fixture so we can test things end-ot-end.
    let fixture = SP1FibonacciProofFixture {
        a,
        b,
        n,
        vkey: vk.bytes32().to_string(),
        public_values: proof.public_values.bytes().to_string(),
        proof: proof.bytes().to_string(),
    };

    // The verification key is used to verify that the proof corresponds to the execution of the
    // program on the given input.
    //
    // Note that the verification key stays the same regardless of the input.
    println!("Verification Key: {}", fixture.vkey);

    // The public values are the values whicha are publically commited to by the zkVM.
    //
    // If you need to expose the inputs or outputs of your program, you should commit them in
    // the public values.
    println!("Public Values: {}", fixture.public_values);

    // The proof proves to the verifier that the program was executed with some inputs that led to
    // the give public values.
    println!("Proof Bytes: {}", fixture.proof);

    // Save the fixture to a file.
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contracts/src/fixtures");
    std::fs::create_dir_all(&fixture_path).expect("failed to create fixture path");
    std::fs::write(
        fixture_path.join("fixture.json"),
        serde_json::to_string_pretty(&fixture).unwrap(),
    )
    .expect("failed to write fixture");
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
