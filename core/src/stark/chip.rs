use core::hash::Hash;

use core::marker::PhantomData;

use crate::lookup::InteractionView;
use crate::lookup::LogupInteraction;
use p3_air::{Air, BaseAir, PairBuilder};
use p3_field::{ExtensionField, Field, PrimeField, PrimeField32};
use p3_matrix::dense::RowMajorMatrix;
use p3_util::log2_ceil_usize;

use crate::{
    air::{MachineAir, MultiTableAirBuilder, SP1AirBuilder},
    lookup::{Interaction, InteractionBuilder},
    runtime::{ExecutionRecord, Program},
};

use super::{
    eval_permutation_constraints, generate_permutation_trace, DebugConstraintBuilder,
    ProverConstraintFolder, RiscvAir, StarkGenericConfig, VerifierConstraintFolder,
};

/// An Air that encodes lookups based on interactions.
pub struct Chip<F, A, I = Interaction<F>, S = Vec<I>> {
    /// The underlying AIR of the chip for constraint evaluation.
    air: A,
    /// The interactions that the chip sends.
    sends: S,
    /// The interactions that the chip receives.
    receives: S,
    /// The relative log degree of the quotient polynomial, i.e. `log2(max_constraint_degree - 1)`.
    log_quotient_degree: usize,
    _marker: PhantomData<(F, I)>,
}

/// A Chip variant whose defults generics are selected as a view of the default Chip struct.
pub type ChipView<'a, F, A, I = InteractionView<'a, F>> = Chip<F, A, I, &'a [I]>;

impl<F: Field, A, I, S> Chip<F, A, I, S>
where
    I: LogupInteraction<F = F>,
    S: AsRef<[I]>,
{
    /// The send interactions of the chip.
    pub fn sends(&self) -> &[I] {
        self.sends.as_ref()
    }

    /// The receive interactions of the chip.
    pub fn receives(&self) -> &[I] {
        self.receives.as_ref()
    }

    /// The relative log degree of the quotient polynomial, i.e. `log2(max_constraint_degree - 1)`.
    pub const fn log_quotient_degree(&self) -> usize {
        self.log_quotient_degree
    }

    pub fn generate_permutation_trace<EF: ExtensionField<F>>(
        &self,
        preprocessed: &Option<RowMajorMatrix<F>>,
        main: &RowMajorMatrix<F>,
        random_elements: &[EF],
    ) -> RowMajorMatrix<EF>
    where
        F: PrimeField,
    {
        generate_permutation_trace(
            self.sends.as_ref(),
            self.receives.as_ref(),
            preprocessed,
            main,
            random_elements,
        )
    }
}

impl<F: PrimeField32, I, S> Chip<F, RiscvAir<F>, I, S>
where
    I: LogupInteraction<F = F>,
    S: AsRef<[I]>,
{
    /// Returns whether the given chip is included in the execution record of the shard.
    pub fn included(&self, shard: &ExecutionRecord) -> bool {
        self.air.included(shard)
    }
}

/// A trait for AIRs that can be used with STARKs.
///
/// This trait is for specifying a trait bound for explicit types of builders used in the stark
/// proving system. It is automatically implemented on any type that implements `Air<AB>` with
/// `AB: SP1AirBuilder`. Users should not need to implement this trait manually.
pub trait StarkAir<SC: StarkGenericConfig>:
    MachineAir<SC::Val>
    + Air<InteractionBuilder<SC::Val>>
    + for<'a> Air<ProverConstraintFolder<'a, SC>>
    + for<'a> Air<VerifierConstraintFolder<'a, SC>>
    + for<'a> Air<DebugConstraintBuilder<'a, SC::Val, SC::Challenge>>
{
}

impl<SC: StarkGenericConfig, T> StarkAir<SC> for T where
    T: MachineAir<SC::Val>
        + Air<InteractionBuilder<SC::Val>>
        + for<'a> Air<ProverConstraintFolder<'a, SC>>
        + for<'a> Air<VerifierConstraintFolder<'a, SC>>
        + for<'a> Air<DebugConstraintBuilder<'a, SC::Val, SC::Challenge>>
{
}

impl<F, A> Chip<F, A>
where
    F: Field,
{
    /// Records the interactions and constraint degree from the air and crates a new chip.
    pub fn new(air: A) -> Self
    where
        A: Air<InteractionBuilder<F>>,
    {
        let mut builder = InteractionBuilder::new(air.width());
        air.eval(&mut builder);
        let (sends, receives) = builder.interactions();

        // TODO: count constraints from the air.
        let max_constraint_degree = 3;
        let log_quotient_degree = log2_ceil_usize(max_constraint_degree - 1);

        Self {
            air,
            sends,
            receives,
            log_quotient_degree,
            _marker: PhantomData,
        }
    }
}

impl<F, A, I, S> BaseAir<F> for Chip<F, A, I, S>
where
    F: Field,
    A: BaseAir<F>,
    I: LogupInteraction<F = F>,
    S: AsRef<[I]> + Sync,
{
    fn width(&self) -> usize {
        self.air.width()
    }

    fn preprocessed_trace(&self) -> Option<RowMajorMatrix<F>> {
        self.air.preprocessed_trace()
    }
}

impl<F, A, I, S> MachineAir<F> for Chip<F, A, I, S>
where
    F: Field,
    A: MachineAir<F>,
    I: LogupInteraction<F = F>,
    S: AsRef<[I]> + Sync,
{
    fn name(&self) -> String {
        self.air.name()
    }
    fn generate_preprocessed_trace(&self, program: &Program) -> Option<RowMajorMatrix<F>> {
        <A as MachineAir<F>>::generate_preprocessed_trace(&self.air, program)
    }

    fn preprocessed_width(&self) -> usize {
        self.air.preprocessed_width()
    }

    fn generate_trace(
        &self,
        input: &ExecutionRecord,
        output: &mut ExecutionRecord,
    ) -> RowMajorMatrix<F> {
        self.air.generate_trace(input, output)
    }

    fn generate_dependencies(&self, input: &ExecutionRecord, output: &mut ExecutionRecord) {
        self.air.generate_dependencies(input, output)
    }
}

// Implement AIR directly on Chip, evaluating both execution and permutation constraints.
impl<F, A, AB, I, S> Air<AB> for Chip<F, A, I, S>
where
    F: Field,
    A: Air<AB>,
    I: LogupInteraction<F = F>,
    S: AsRef<[I]> + Sync,
    AB: SP1AirBuilder<F = F> + MultiTableAirBuilder + PairBuilder,
{
    fn eval(&self, builder: &mut AB) {
        // Evaluate the execution trace constraints.
        self.air.eval(builder);
        // Evaluate permutation constraints.
        eval_permutation_constraints(self.sends.as_ref(), self.receives.as_ref(), builder);
    }
}

impl<F, A, I, S> PartialEq for Chip<F, A, I, S>
where
    F: Field,
    A: PartialEq,
    I: LogupInteraction<F = F>,
    S: AsRef<[I]> + Sync,
{
    fn eq(&self, other: &Self) -> bool {
        self.air == other.air
    }
}

impl<F: Field, A: Eq, I, S> Eq for Chip<F, A, I, S>
where
    F: Field + Eq,
    I: LogupInteraction<F = F>,
    S: AsRef<[I]> + Sync,
{
}

impl<F, A, I, S> Hash for Chip<F, A, I, S>
where
    F: Field,
    A: Hash,
    I: LogupInteraction<F = F>,
    S: AsRef<[I]> + Sync,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.air.hash(state);
    }
}
