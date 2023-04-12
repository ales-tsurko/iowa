//! Operator table.

use std::{collections::HashMap, sync::Mutex};

use dyn_clone::DynClone;
use once_cell::sync::OnceCell;

/// A table of operators.
#[derive(Debug)]
pub struct OperatorTable {
    table: Mutex<HashMap<&'static str, Box<dyn Operator>>>,
}

impl OperatorTable {
    /// Get the global operator table.
    pub fn global() -> &'static Self {
        static INSTANCE: OnceCell<OperatorTable> = OnceCell::new();
        INSTANCE.get_or_init(|| Self::default())
    }

    /// Update/insert an operator in the table.
    pub fn add_operator(operator: impl Operator) {
        Self::global()
            .table
            .lock()
            .unwrap()
            .insert(operator.symbol(), Box::new(operator));
    }

    /// Get an operator from the table.
    pub fn get(symbol: &str) -> Option<Box<dyn Operator>> {
        Self::global().table.lock().unwrap().get(symbol).cloned()
    }

    /// Get all symbols in the table.
    pub fn all_symbols() -> Vec<&'static str> {
        Self::global()
            .table
            .lock()
            .unwrap()
            .keys()
            .copied()
            .collect()
    }
}

impl Default for OperatorTable {
    fn default() -> Self {
        let table: HashMap<_, _> = [
            (
                QuestionMark.symbol(),
                Box::new(QuestionMark) as Box<dyn Operator>,
            ),
            (At.symbol(), Box::new(At)),
            (AtAt.symbol(), Box::new(AtAt)),
            (Power.symbol(), Box::new(Power)),
            (Modulo.symbol(), Box::new(Modulo)),
            (Multiply.symbol(), Box::new(Multiply)),
            (Divide.symbol(), Box::new(Divide)),
            (Plus.symbol(), Box::new(Plus)),
            (Minus.symbol(), Box::new(Minus)),
            (ShiftLeft.symbol(), Box::new(ShiftLeft)),
            (ShiftRight.symbol(), Box::new(ShiftRight)),
            (LessThan.symbol(), Box::new(LessThan)),
            (LessThanEquals.symbol(), Box::new(LessThanEquals)),
            (GreaterThan.symbol(), Box::new(GreaterThan)),
            (GreaterThanEquals.symbol(), Box::new(GreaterThanEquals)),
            (NotEquals.symbol(), Box::new(NotEquals)),
            (EqualsEquals.symbol(), Box::new(EqualsEquals)),
            (BitwiseAnd.symbol(), Box::new(BitwiseAnd)),
            (BitwiseXor.symbol(), Box::new(BitwiseXor)),
            (BitwiseOr.symbol(), Box::new(BitwiseOr)),
            (And.symbol(), Box::new(And)),
            (AndKey.symbol(), Box::new(AndKey)),
            (Or.symbol(), Box::new(Or)),
            (OrKey.symbol(), Box::new(OrKey)),
            (DotDot.symbol(), Box::new(DotDot)),
            (Assign.symbol(), Box::new(Assign)),
            (ColonAssign.symbol(), Box::new(ColonAssign)),
            (ColonColonAssign.symbol(), Box::new(ColonColonAssign)),
            (ModuloAssign.symbol(), Box::new(ModuloAssign)),
            (MultiplyAssign.symbol(), Box::new(MultiplyAssign)),
            (DivideAssign.symbol(), Box::new(DivideAssign)),
            (PlusAssign.symbol(), Box::new(PlusAssign)),
            (MinusAssign.symbol(), Box::new(MinusAssign)),
            (ShiftLeftAssign.symbol(), Box::new(ShiftLeftAssign)),
            (ShiftRightAssign.symbol(), Box::new(ShiftRightAssign)),
            (BitwiseAndAssign.symbol(), Box::new(BitwiseAndAssign)),
            (BitwiseXorAssign.symbol(), Box::new(BitwiseXorAssign)),
            (BitwiseOrAssign.symbol(), Box::new(BitwiseOrAssign)),
            (Return.symbol(), Box::new(Return)),
        ]
        .into();

        Self {
            table: Mutex::new(table),
        }
    }
}

/// Each operator should implement this trait.
pub trait Operator: std::fmt::Debug + DynClone + Send + Sync + 'static {
    /// The operator symbol (`=`, `>`, etc.).
    fn symbol(&self) -> &'static str;
    /// The operator precedence.
    fn precedence(&self) -> u32;
}

dyn_clone::clone_trait_object!(Operator);

macro_rules! impl_op {
    ($name:ident, $symbol:expr, $precedence:expr) => {
        #[derive(Debug, Clone, Copy)]
        #[allow(missing_docs)]
        pub struct $name;

        impl Operator for $name {
            fn symbol(&self) -> &'static str {
                $symbol
            }

            fn precedence(&self) -> u32 {
                $precedence
            }
        }
    };
}

impl_op!(QuestionMark, "?", 0);
impl_op!(At, "@", 0);
impl_op!(AtAt, "@@", 0);
impl_op!(Power, "**", 1);
impl_op!(Modulo, "%", 2);
impl_op!(Multiply, "*", 2);
impl_op!(Divide, "/", 2);
impl_op!(Plus, "+", 3);
impl_op!(Minus, "-", 3);
impl_op!(ShiftLeft, "<<", 4);
impl_op!(ShiftRight, ">>", 4);
impl_op!(LessThan, "<", 5);
impl_op!(LessThanEquals, "<=", 5);
impl_op!(GreaterThan, ">", 5);
impl_op!(GreaterThanEquals, ">=", 5);
impl_op!(NotEquals, "!=", 6);
impl_op!(EqualsEquals, "==", 6);
impl_op!(BitwiseAnd, "&", 7);
impl_op!(BitwiseXor, "^", 8);
impl_op!(BitwiseOr, "|", 9);
impl_op!(And, "&&", 10);
impl_op!(AndKey, "and", 10);
impl_op!(Or, "||", 11);
impl_op!(OrKey, "or", 11);
impl_op!(DotDot, "..", 12);
impl_op!(Assign, "=", 13);
impl_op!(ColonAssign, ":=", 13);
impl_op!(ColonColonAssign, "::=", 13);
impl_op!(ModuloAssign, "%=", 13);
impl_op!(MultiplyAssign, "*=", 13);
impl_op!(DivideAssign, "/=", 13);
impl_op!(PlusAssign, "+=", 13);
impl_op!(MinusAssign, "-=", 13);
impl_op!(ShiftLeftAssign, "<<=", 13);
impl_op!(ShiftRightAssign, ">>=", 13);
impl_op!(BitwiseAndAssign, "&=", 13);
impl_op!(BitwiseXorAssign, "^=", 13);
impl_op!(BitwiseOrAssign, "|=", 13);
impl_op!(Return, "return", u32::MAX);
