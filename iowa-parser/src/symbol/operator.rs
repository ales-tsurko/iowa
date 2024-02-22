//! Operator table.

use std::sync::Mutex;
use std::sync::OnceLock;

use dyn_clone::DynClone;
use rayon::prelude::*;

/// A table of operators.
#[derive(Debug)]
pub struct OperatorTable {
    // we very rarely push operators and we access precedence during parse directly from a concrete
    // operator, but we need to preserve order of insertion to prevent ambiguity during parsing, so
    // it's better to use Vec instead of HashMap here
    pub(crate) table: Mutex<Vec<Box<dyn Operator>>>,
}

impl OperatorTable {
    /// Get the global operator table.
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<OperatorTable> = OnceLock::new();
        INSTANCE.get_or_init(|| Self::default())
    }

    /// Add an operator to the table (if it's not there already).
    pub fn add_operator(operator: impl Operator) {
        let mut table = Self::global().table.lock().unwrap();
        if table
            .binary_search_by_key(&operator.symbol(), |op| op.symbol())
            .is_err()
        {
            table.push(Box::new(operator));
            table.sort_unstable_by_key(|op| op.symbol());
            table.reverse();
        }
    }
}

impl Default for OperatorTable {
    fn default() -> Self {
        let mut table = vec![
            Box::new(QuestionMark) as Box<dyn Operator>,
            Box::new(AtAt),
            Box::new(ColonAssign),
            Box::new(ColonColonAssign),
            Box::new(ModuloAssign),
            Box::new(MultiplyAssign),
            Box::new(DivideAssign),
            Box::new(PlusAssign),
            Box::new(MinusAssign),
            Box::new(ShiftLeftAssign),
            Box::new(ShiftRightAssign),
            Box::new(BitwiseAndAssign),
            Box::new(BitwiseXorAssign),
            Box::new(BitwiseOrAssign),
            Box::new(Power),
            Box::new(At),
            Box::new(Modulo),
            Box::new(Multiply),
            Box::new(Divide),
            Box::new(Plus),
            Box::new(Minus),
            Box::new(ShiftLeft),
            Box::new(ShiftRight),
            Box::new(LessThanEquals),
            Box::new(GreaterThanEquals),
            Box::new(LessThan),
            Box::new(GreaterThan),
            Box::new(NotEquals),
            Box::new(Assign),
            Box::new(EqualsEquals),
            Box::new(And),
            Box::new(BitwiseAnd),
            Box::new(BitwiseXor),
            Box::new(BitwiseOr),
            Box::new(AndKey),
            Box::new(Or),
            Box::new(OrKey),
            Box::new(DotDot),
            Box::new(Return),
        ];
        table.par_sort_unstable_by_key(|op| op.symbol());
        table.reverse();

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
