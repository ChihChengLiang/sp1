use std::borrow::BorrowMut;

use alloc::vec::Vec;

use itertools::Itertools;
use p3_field::{
    extension::{BinomialExtensionField, BinomiallyExtendable},
    AbstractExtensionField, PrimeField32,
};
use p3_matrix::dense::RowMajorMatrix;
use p3_maybe_rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use tracing::instrument;

use crate::{
    air::{MachineAir, DEGREE},
    cpu::MemoryRecordEnum,
    field::event::FieldEvent,
    runtime::ExecutionRecord,
    utils::pad_rows,
};

use super::{
    columns::{
        FriFoldCols, ALPHA_POW_ADDR_IDX, NUM_FRI_FOLD_COLS, NUM_INPUT_ELMS, NUM_OUTPUT_ELMS,
        RO_ADDR_IDX,
    },
    FriFoldChip,
};

impl<F: PrimeField32 + BinomiallyExtendable<4>> MachineAir<F> for FriFoldChip {
    fn name(&self) -> String {
        "FriFold".to_string()
    }

    #[instrument(name = "generate fri fold trace", skip_all)]
    fn generate_trace(
        &self,
        input: &ExecutionRecord,
        output: &mut ExecutionRecord,
    ) -> RowMajorMatrix<F> {
        let (mut rows, new_field_events_list): (Vec<[F; NUM_FRI_FOLD_COLS]>, Vec<Vec<FieldEvent>>) =
            input
                .fri_fold_events
                .par_iter()
                .map(|event| {
                    // Enter in data for the read row
                    let mut input_row = [F::zero(); NUM_FRI_FOLD_COLS];
                    let input_cols: &mut FriFoldCols<F> = input_row.as_mut_slice().borrow_mut();

                    // Populate basic columns.
                    input_cols.is_real = F::one();
                    input_cols.is_input = F::one();
                    input_cols.shard = F::from_canonical_u32(event.shard);
                    input_cols.clk = F::from_canonical_u32(event.clk);
                    input_cols.input_slice_ptr = F::from_canonical_u32(event.input_slice_ptr);
                    input_cols.output_slice_ptr = F::from_canonical_u32(event.output_slice_ptr);

                    println!(
                        "input_slice is {:?}",
                        event
                            .input_slice_read_records
                            .iter()
                            .map(|x| x.value)
                            .collect_vec()
                    );

                    let num_base_elms = event.num.map(F::from_canonical_u32);
                    let num = BinomialExtensionField::from_base_slice(&num_base_elms);
                    let denom_base_elms = event.denom.map(F::from_canonical_u32);
                    let denom = BinomialExtensionField::from_base_slice(&denom_base_elms);

                    input_cols.div_ext_op.populate(num, denom);

                    // Populate the memory access columns.
                    let mut input_new_field_events = Vec::new();
                    for i in 0..NUM_INPUT_ELMS {
                        input_cols.input_slice_read_records[i].populate(
                            event.input_slice_read_records[i],
                            &mut input_new_field_events,
                        );
                    }
                    for i in 0..NUM_OUTPUT_ELMS {
                        input_cols.output_slice_read_records[i].populate(
                            event.output_slice_read_records[i],
                            &mut input_new_field_events,
                        );
                    }

                    input_cols.ro_addr =
                        F::from_canonical_u32(event.output_slice_read_records[RO_ADDR_IDX].value);
                    input_cols.alpha_pow_addr = F::from_canonical_u32(
                        event.output_slice_read_records[ALPHA_POW_ADDR_IDX].value,
                    );

                    println!(
                        "ro_read is {:?}",
                        event.ro_read_records.iter().map(|x| x.value).collect_vec()
                    );

                    println!(
                        "alpha_pow_read is {:?}",
                        event
                            .alpha_pow_read_records
                            .iter()
                            .map(|x| x.value)
                            .collect_vec()
                    );

                    println!(
                        "ro_write is {:?}",
                        event.ro_write_records.iter().map(|x| x.value).collect_vec()
                    );

                    println!(
                        "alpha_pow_write is {:?}",
                        event
                            .alpha_pow_write_records
                            .iter()
                            .map(|x| x.value)
                            .collect_vec()
                    );

                    for i in 0..DEGREE {
                        input_cols.ro_rw_records[i].populate(
                            MemoryRecordEnum::Read(event.ro_read_records[i]),
                            &mut input_new_field_events,
                        );
                        input_cols.alpha_pow_rw_records[i].populate(
                            MemoryRecordEnum::Read(event.alpha_pow_read_records[i]),
                            &mut input_new_field_events,
                        );
                    }

                    let mut output_row = [F::zero(); NUM_FRI_FOLD_COLS];
                    let output_cols: &mut FriFoldCols<F> = output_row.as_mut_slice().borrow_mut();
                    output_cols.is_real = F::one();
                    output_cols.is_input = F::zero();
                    output_cols.shard = F::from_canonical_u32(event.shard);
                    output_cols.clk = F::from_wrapped_u32(event.clk) + F::from_wrapped_u32(4);
                    output_cols.ro_addr = input_cols.ro_addr;
                    output_cols.alpha_pow_addr = input_cols.alpha_pow_addr;

                    let mut output_new_field_events = Vec::new();
                    for i in 0..DEGREE {
                        output_cols.ro_rw_records[i].populate(
                            MemoryRecordEnum::Write(event.ro_write_records[i]),
                            &mut output_new_field_events,
                        );
                        output_cols.alpha_pow_rw_records[i].populate(
                            MemoryRecordEnum::Write(event.alpha_pow_write_records[i]),
                            &mut output_new_field_events,
                        );
                    }

                    [
                        (input_row, input_new_field_events),
                        (output_row, output_new_field_events),
                    ]
                })
                .flatten()
                .unzip();

        for new_field_events in new_field_events_list {
            output.add_field_events(&new_field_events);
        }

        pad_rows(&mut rows, || [F::zero(); NUM_FRI_FOLD_COLS]);

        // Convert the trace to a row major matrix.
        RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_FRI_FOLD_COLS,
        )
    }
}