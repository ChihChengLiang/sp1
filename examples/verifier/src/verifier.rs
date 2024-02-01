#![no_main]

extern crate succinct_zkvm;

use std::fs;
use std::hint::black_box;

// use clap::{command, Parser};
use p3_commit::Pcs;
use p3_field::{ExtensionField, PrimeField, PrimeField32, TwoAdicField};
use p3_matrix::dense::RowMajorMatrix;

use succinct_core::runtime::Program;
use succinct_core::runtime::Runtime;
use succinct_core::stark::types::SegmentProof;
use succinct_core::stark::StarkConfig;
use succinct_core::utils::BabyBearPoseidon2;
use succinct_core::utils::StarkUtils;

succinct_zkvm::entrypoint!(main);

// #[derive(Parser, Debug, Clone)]
// #[command(about = "Profile a program.")]
// struct VerifierArgs {
//     #[arg(long)]
//     pub program: String,

//     #[arg(long)]
//     pub proof_directory: String,
// }

#[succinct_derive::cycle_tracker]
fn verify<F, EF, SC>(
    runtime: &mut Runtime,
    config: &SC,
    challenger: &mut SC::Challenger,
    segment_proofs: &[SegmentProof<SC>],
    global_proof: &SegmentProof<SC>,
) where
    F: PrimeField + TwoAdicField + PrimeField32,
    EF: ExtensionField<F>,
    SC: StarkConfig<Val = F, Challenge = EF> + Send + Sync,
    SC::Challenger: Clone,
    <SC::Pcs as Pcs<SC::Val, RowMajorMatrix<SC::Val>>>::Commitment: Send + Sync,
    <SC::Pcs as Pcs<SC::Val, RowMajorMatrix<SC::Val>>>::ProverData: Send + Sync,
{
    runtime
        .verify::<_, _, SC>(config, challenger, segment_proofs, global_proof)
        .unwrap();
}

fn main() {
    // let args = VerifierArgs::parse();

    // log::info!("Verifying proof: {}", args.proof_directory.as_str());

    let proof_directory = "verifier/fib_proofs";
    let segment_proofs: Vec<SegmentProof<BabyBearPoseidon2>> = {
        let segment_proofs_file_name = format!("{}/segment_proofs.json", proof_directory);
        let segment_proofs_json = fs::read_to_string(segment_proofs_file_name).unwrap();
        serde_json::from_str(&segment_proofs_json).unwrap()
    };

    let global_proof = {
        let global_proof_file_name = format!("{}/global_proof.json", proof_directory);
        let global_proof_json = fs::read_to_string(global_proof_file_name).unwrap();
        serde_json::from_str(&global_proof_json).unwrap()
    };

    let config = BabyBearPoseidon2::new();
    let mut challenger = config.challenger();

    let program = Program::from_elf("../../programs/fibonacci");
    let mut runtime = Runtime::new(program);
    black_box(verify::<_, _, BabyBearPoseidon2>(
        black_box(&mut runtime),
        black_box(&config),
        black_box(&mut challenger),
        black_box(&segment_proofs),
        black_box(&global_proof),
    ));
}
