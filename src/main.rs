use ark_bls12_381::{Bls12_381, Fr};
use ark_marlin::Marlin;
use ark_poly::univariate::DensePolynomial;
use ark_poly_commit::sonic_pc::SonicKZG10;
use ark_relations::lc;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable};
use blake2::Blake2s;

use ark_serialize::CanonicalSerialize;
use ark_std::time::Instant;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rdrand::RdRand;

/*
    We cannot use thread_rng() inside an enclave because it relies on OS. https://en.wikipedia.org/wiki/RDRAND.
    randomness (e.g./dev/urandom). In SGX, the recommended entropy source
    is the CPU hardware RNG (RDRAND / RDSEED), exposed by the Intel SGX SDK via sgx_read_rand().

    DEV API REF https://cdrdv2-public.intel.com/671508/sgx-sdk-developer-reference-for-windows-os.pdf
    https://download.01.org/intel-sgx/latest/linux-latest/docs/Intel_SGX_Developer_Guide.pdf
    Example https://github.com/fortanix/rust-sgx/blob/master/examples/tls/src/main.rs
    However, mbedtls::rng::Rdrand is designed for mbedtls (something like own trait for RNG?)

    Here we use rdrand::RdRand to get 32 bytes of hardware entropy and then
    seed ChaCha20Rng, which implements RngCore + CryptoRng and is accepted by arkworks/Marlin as a cryptographically secure RNG:)
*/

fn sgx_rng() -> ChaCha20Rng {
    let mut hardware_enthropy = RdRand::new().expect("RdRand not available");
    let seed: [u8; 32] = hardware_enthropy.gen();
    ChaCha20Rng::from_seed(seed)
}
// Fr - scalar Finite Field - Fq256
// Blake2s is used for Fiat Shamir transformation. Why Blake? Because https://github.com/arkworks-rs/marlin/blob/master/benches/bench.rs
// However blake2s is secure.


// Our circuit y = a*x + b
pub struct LinearCircuit {
    pub x: Option<Fr>, // witness - private
    pub y: Option<Fr>, // public input
    pub a: Fr,
    pub b: Fr,
}

impl ConstraintSynthesizer<Fr> for LinearCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // Difference from Bellman is that out witness - private input has to be initialized with new_witness_variable
        let x = cs.new_witness_variable(|| self.x.ok_or(SynthesisError::AssignmentMissing))?;

        // new_input_variable bulic inputy
        let y = cs.new_input_variable(|| self.y.ok_or(SynthesisError::AssignmentMissing))?;

        // tmp = a * x
        let tmp_val = self.x.map(|mut v| {
            v *= self.a;
            v
        });

        let tmp = cs.new_witness_variable(|| tmp_val.ok_or(SynthesisError::AssignmentMissing))?;

        // constraint: a * x = tmp
        cs.enforce_constraint(lc!() + (self.a, x), lc!() + Variable::One, lc!() + tmp)?;

        // constraint: tmp + b = y
        cs.enforce_constraint(
            lc!() + tmp + (self.b, Variable::One),
            lc!() + Variable::One,
            lc!() + y,
        )?;

        Ok(())
    }
}

fn main() {
    println!("Running linear Marlin test: y = a*x + b inside SGX...");

    let a = Fr::from(3u64);
    let b = Fr::from(5u64);

    // x = 11 and linear fun is  y = 3*11 + 5 = 38
    let x_linear_value = Fr::from(11u64);
    let y_linear_value = Fr::from(38u64);

    // Like here https://github.com/arkworks-rs/marlin/blob/master/benches/bench.rs we need to create 
    let num_constraints = 2; // 2 calls of enforce_constraint =2
    let num_variables = 3; // 1 witnes + 1 public inputy + tmp temporary witness  =3
    // but they are upper limit. 

    let mut rng: ChaCha20Rng = sgx_rng();

    let srs_start = Instant::now();
    let universal_srs = Marlin::<
        Fr,
        SonicKZG10<Bls12_381, DensePolynomial<Fr>>,
        Blake2s
    >::universal_setup(
        num_constraints,
        num_variables,
        3 * num_constraints,
        &mut rng,
    )
    .expect("universal setup failed");

    let srs_time = srs_start.elapsed();
    println!("Universal setup time: {:?}", srs_time);

    // like in Bellman empty circuit to generate pk and vk
    let empty_circuit = LinearCircuit {
        x: None,
        y: None,
        a,
        b,
    };

    let index_start = Instant::now();
    let (pk, vk) = Marlin::<
        Fr,
        SonicKZG10<Bls12_381, DensePolynomial<Fr>>,
        Blake2s
    >::index(&universal_srs, empty_circuit).expect("indexing failed"); // TODO: CREATE type MerlinInstance with this ugly generic and replace it everywhere
    let index_time = index_start.elapsed();
    println!("pk and vk generation time: {:?}", index_time);

    // proving
    let circuit = LinearCircuit {
        x: Some(x_linear_value),
        y: Some(y_linear_value),
        a,
        b,
    };

    let mut rng = sgx_rng();

    let prove_start = Instant::now();
    let proof = Marlin::<
        Fr,
        SonicKZG10<Bls12_381, DensePolynomial<Fr>>,
        Blake2s
    >::prove(&pk, circuit, &mut rng).expect("proving failed");
    let prove_time = prove_start.elapsed();

    // let's check proof in bytes
    let mut proof_bytes = Vec::new();
    proof
        .serialize(&mut proof_bytes)
        .expect("serialize proof failed");

    let proof_size = proof_bytes.len();

    println!("Proving time: {:?}, proof size: {} bytes", prove_time, proof_size);

    // verif
    let public_inputs = vec![y_linear_value];

    let mut rng = sgx_rng();

    let verify_start = Instant::now();
    let ok = Marlin::<
        Fr,
        SonicKZG10<Bls12_381, DensePolynomial<Fr>>,
        Blake2s
    >::verify(&vk, &public_inputs, &proof, &mut rng)
        .expect("verification failed");
    let verify_time = verify_start.elapsed();

    println!("Verification result: {}", ok);
    println!("Verification time: {:?}", verify_time); 
}
