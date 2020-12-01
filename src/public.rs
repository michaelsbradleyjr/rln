use crate::circuit::poseidon::PoseidonCircuit;
use crate::circuit::rln::{RLNCircuit, RLNInputs};
use crate::merkle::MerkleTree;
use crate::poseidon::{Poseidon as PoseidonHasher, PoseidonParams};
use crate::utils::{read_uncompressed_proof, write_uncompressed_proof};
use bellman::groth16::generate_random_parameters;
use bellman::groth16::{create_proof, prepare_verifying_key, verify_proof};
use bellman::groth16::{create_random_proof, Parameters, Proof};
// use bellman::pairing::bn256::{E, Fr, G1Affine, G2Affine};
use bellman::pairing::ff::{Field, PrimeField, PrimeFieldRepr};
use bellman::pairing::{CurveAffine, EncodedPoint, Engine};
use bellman::{Circuit, ConstraintSystem, SynthesisError};
use rand::{Rand, SeedableRng, XorShiftRng};
use std::io::{self, Error, ErrorKind, Read, Write};

pub struct RLN<E>
where
    E: Engine,
{
    circuit_parameters: Parameters<E>,
    circuit_hasher: PoseidonCircuit<E>,
    poseidon_params: PoseidonParams<E>,
    merkle_depth: usize,
}

impl<E> RLN<E>
where
    E: Engine,
{
    fn default_poseidon_params() -> PoseidonParams<E> {
        PoseidonParams::<E>::new(8, 55, 3, None, None, None)
    }

    fn new_circuit(merkle_depth: usize, poseidon_params: PoseidonParams<E>) -> Parameters<E> {
        let mut rng = XorShiftRng::from_seed([0x3dbe6258, 0x8d313d76, 0x3237db17, 0xe5bc0654]);
        let inputs = RLNInputs::<E>::empty(merkle_depth);
        let circuit = RLNCircuit::<E> {
            inputs,
            hasher: PoseidonCircuit::new(poseidon_params.clone()),
        };
        generate_random_parameters(circuit, &mut rng).unwrap()
    }

    fn new_with_params(
        merkle_depth: usize,
        circuit_parameters: Parameters<E>,
        poseidon_params: PoseidonParams<E>,
    ) -> RLN<E> {
        let circuit_hasher = PoseidonCircuit::new(poseidon_params.clone());
        RLN {
            circuit_parameters,
            circuit_hasher,
            poseidon_params,
            merkle_depth,
        }
    }

    pub fn poseidon_params(&self) -> PoseidonParams<E> {
        self.poseidon_params.clone()
    }

    pub fn new(merkle_depth: usize, poseidon_params: Option<PoseidonParams<E>>) -> RLN<E> {
        let poseidon_params = match poseidon_params {
            Some(params) => params,
            None => Self::default_poseidon_params(),
        };
        let circuit_parameters = Self::new_circuit(merkle_depth, poseidon_params.clone());
        Self::new_with_params(merkle_depth, circuit_parameters, poseidon_params)
    }

    pub fn new_with_raw_params<R: Read>(
        merkle_depth: usize,
        raw_circuit_parameters: R,
        poseidon_params: Option<PoseidonParams<E>>,
    ) -> io::Result<RLN<E>> {
        let circuit_parameters = Parameters::<E>::read(raw_circuit_parameters, true)?;
        let poseidon_params = match poseidon_params {
            Some(params) => params,
            None => Self::default_poseidon_params(),
        };
        Ok(Self::new_with_params(
            merkle_depth,
            circuit_parameters,
            poseidon_params,
        ))
    }

    pub fn hasher(&self) -> PoseidonHasher<E> {
        PoseidonHasher::new(self.poseidon_params.clone())
    }

    pub fn generate_proof<R: Read>(&self, input: R) -> io::Result<Vec<u8>> {
        use rand::chacha::ChaChaRng;
        use rand::SeedableRng;
        let mut rng = ChaChaRng::new_unseeded();
        let inputs = RLNInputs::<E>::read(input)?;
        assert_eq!(self.merkle_depth, inputs.merkle_depth());
        let circuit = RLNCircuit {
            inputs: inputs.clone(),
            hasher: self.circuit_hasher.clone(),
        };
        let proof = create_random_proof(circuit, &self.circuit_parameters, &mut rng).unwrap();
        let mut output: Vec<u8> = Vec::new();

        write_uncompressed_proof(proof, &mut output)?;
        // proof.write(&mut output).unwrap();

        Ok(output)
    }

    pub fn verify<R: Read>(&self, uncompresed_proof: R, raw_public_inputs: R) -> io::Result<bool> {
        let proof = read_uncompressed_proof(uncompresed_proof)?;
        // let proof = Proof::read(uncompresed_proof).unwrap();
        let public_inputs = RLNInputs::<E>::read_public_inputs(raw_public_inputs)?;
        let verifing_key = prepare_verifying_key(&self.circuit_parameters.vk);
        let success = verify_proof(&verifing_key, &proof, &public_inputs).unwrap();
        Ok(success)
    }

    pub fn export_verifier_key<W: Write>(&self, w: W) -> io::Result<()> {
        self.circuit_parameters.vk.write(w)
    }

    pub fn export_circuit_parameters<W: Write>(&self, w: W) -> io::Result<()> {
        self.circuit_parameters.write(w)
    }
}
