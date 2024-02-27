use core::fmt::Debug;
use core::fmt::Display;
use p3_air::VirtualPairCol;
use p3_field::Field;
use std::marker::PhantomData;

use crate::air::VirtualColumn;
use crate::air::VirtualPairColView;

pub trait LogupInteraction: Sync {
    type F: Field;
    type VirtualCol: VirtualColumn<Self::F>;

    fn argument_index(&self) -> usize;

    fn kind(&self) -> InteractionKind;

    fn values(&self) -> &[Self::VirtualCol];

    fn multiplicity(&self) -> &Self::VirtualCol;
}

/// An interaction for a lookup or a permutation argument.
pub struct Interaction<F: Field, C = VirtualPairCol<F>> {
    values: Vec<C>,
    multiplicity: C,
    kind: InteractionKind,
    _marker: PhantomData<F>,
}

pub struct InteractionView<'a, F, C = VirtualPairColView<'a, F>> {
    pub values: &'a [C],
    pub multiplicity: C,
    pub kind: InteractionKind,
    _marker: PhantomData<F>,
}

/// The type of interaction for a lookup argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InteractionKind {
    /// Interaction with the memory table, such as read and write.
    Memory = 1,

    /// Interaction with the program table, loading an instruction at a given pc address.
    Program = 2,

    /// Interaction with instruction oracle.
    Instruction = 3,

    /// Interaction with the ALU operations
    Alu = 4,

    /// Interaction with the byte lookup table for byte operations.
    Byte = 5,

    /// Requesting a range check for a given value and range.
    Range = 6,

    /// Interaction with the field op table for field operations.
    Field = 7,
}

impl InteractionKind {
    pub fn all_kinds() -> Vec<InteractionKind> {
        vec![
            InteractionKind::Memory,
            InteractionKind::Program,
            InteractionKind::Instruction,
            InteractionKind::Alu,
            InteractionKind::Byte,
            InteractionKind::Range,
            InteractionKind::Field,
        ]
    }
}

impl<F: Field, C: VirtualColumn<F>> Interaction<F, C> {
    /// Create a new interaction.
    pub fn new(values: Vec<C>, multiplicity: C, kind: InteractionKind) -> Self {
        Self {
            values,
            multiplicity,
            kind,
            _marker: PhantomData,
        }
    }

    /// The index of the argument in the lookup table.
    pub fn argument_index(&self) -> usize {
        self.kind as usize
    }
}

impl<'a, F: Field, C: VirtualColumn<F>> InteractionView<'a, F, C> {
    /// Create a new interaction.
    pub fn new(values: &'a [C], multiplicity: C, kind: InteractionKind) -> Self {
        Self {
            values,
            multiplicity,
            kind,
            _marker: PhantomData,
        }
    }

    /// The index of the argument in the lookup table.
    pub fn argument_index(&self) -> usize {
        self.kind as usize
    }
}

impl<F: Field, C: VirtualColumn<F>> LogupInteraction for Interaction<F, C> {
    type F = F;
    type VirtualCol = C;

    fn argument_index(&self) -> usize {
        self.kind as usize
    }

    fn values(&self) -> &[C] {
        &self.values
    }

    fn multiplicity(&self) -> &C {
        &self.multiplicity
    }

    fn kind(&self) -> InteractionKind {
        self.kind
    }
}

// TODO: add debug for VirtualPairCol so that we can derive Debug for Interaction.
impl<F: Field> Debug for Interaction<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interaction")
            .field("kind", &self.kind)
            .finish()
    }
}

impl Display for InteractionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InteractionKind::Memory => write!(f, "Memory"),
            InteractionKind::Program => write!(f, "Program"),
            InteractionKind::Instruction => write!(f, "Instruction"),
            InteractionKind::Alu => write!(f, "Alu"),
            InteractionKind::Byte => write!(f, "Byte"),
            InteractionKind::Range => write!(f, "Range"),
            InteractionKind::Field => write!(f, "Field"),
        }
    }
}

impl<'a, F: Field, C: VirtualColumn<F>> LogupInteraction for InteractionView<'a, F, C> {
    type F = F;
    type VirtualCol = C;

    fn argument_index(&self) -> usize {
        self.kind as usize
    }

    fn values(&self) -> &[C] {
        self.values
    }

    fn multiplicity(&self) -> &C {
        &self.multiplicity
    }

    fn kind(&self) -> InteractionKind {
        self.kind
    }
}
