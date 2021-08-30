use crate::expression::flat_details::{self, FlatNodeVec, FlatOpVec};
use crate::Operator;
use crate::{
    expression::{
        deep::{DeepBuf, DeepEx, ExprIdxVec},
        partial_derivatives,
    },
    operators,
    parser::ExParseError,
};
use num::Float;
use std::fmt::{self, Debug, Display, Formatter};
use std::str::FromStr;

/// Flattens a deep expression
/// The result does not contain any recursive structures and is faster to evaluate.
pub fn flatten<T: Copy + Debug>(deepex: DeepEx<T>) -> FlatEx<T> {
    let (nodes, ops) = flat_details::flatten_vecs(&deepex, 0);
    let indices = flat_details::prioritized_indices_flat(&ops, &nodes);
    let n_unique_vars = deepex.n_vars();
    FlatEx {
        nodes,
        ops,
        prio_indices: indices,
        n_unique_vars,
        deepex: Some(deepex),
    }
}

/// This is the core data type representing a flattened expression and the result of
/// parsing a string. We use flattened expressions to make efficient evaluation possible.
/// Simplified, a flat expression consists of a [`SmallVec`](https://docs.rs/smallvec/)
/// of nodes and a [`SmallVec`](https://docs.rs/smallvec/) of operators that are applied
/// to the nodes in an order following operator priorities.
///
/// You create an expression with the `parse` function or one of its
/// variants, namely `parse_with_default_ops` and `parse_with_number_pattern`.
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// #
/// use exmex::prelude::*;
///
/// // create an expression by parsing a string
/// let expr = FlatEx::<f32>::from_str("sin(1+y)*x")?;
/// assert!((expr.eval(&[1.5, 2.0])? - (1.0 + 2.0 as f32).sin() * 1.5).abs() < 1e-6);
/// #
/// #     Ok(())
/// # }
/// ```
/// The second argument `&[1.5, 2.0]` in the call of [`eval`](FlatEx::eval) specifies the
/// variable values in the alphabetical order of the variable names.
/// In this example, we want to evaluate the expression for the varibale values `x=2.0` and `y=1.5`.
/// Variables in the string to-be-parsed are all substrings that are no numbers, no
/// operators, and no parentheses.
///
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct FlatEx<'a, T: Copy + Debug> {
    nodes: FlatNodeVec<T>,
    ops: FlatOpVec<T>,
    prio_indices: ExprIdxVec,
    n_unique_vars: usize,
    deepex: Option<DeepEx<'a, T>>,
}

impl<'a, T: Copy + Debug> Expression<'a, T> for FlatEx<'a, T> {
    fn from_str(text: &'a str) -> Result<Self, ExParseError>
    where
        <T as std::str::FromStr>::Err: Debug,
        T: Float + FromStr,
    {
        Ok(flatten(DeepEx::from_str(text)?))
    }

    fn from_ops(text: &'a str, ops: &[Operator<'a, T>]) -> Result<Self, ExParseError>
    where
        <T as std::str::FromStr>::Err: Debug,
        T: Copy + FromStr + Debug,
    {
        let deepex = DeepEx::from_ops(text, ops)?;
        Ok(flatten(deepex))
    }

    fn from_pattern(
        text: &'a str,
        ops: &[Operator<'a, T>],
        number_regex_pattern: &str,
    ) -> Result<Self, ExParseError>
    where
        <T as std::str::FromStr>::Err: Debug,
        T: Copy + FromStr + Debug,
    {
        let deepex = DeepEx::from_pattern(text, ops, number_regex_pattern)?;
        Ok(flatten(deepex))
    }

    fn eval(&self, vars: &[T]) -> Result<T, ExParseError> {
        flat_details::eval_flatex(
            vars,
            &self.nodes,
            &self.ops,
            &self.prio_indices,
            self.n_unique_vars,
        )
    }
    fn partial(self, var_idx: usize) -> Result<Self, ExParseError>
    where
        T: Float,
    {
        let ops = operators::make_default_operators();

        let d_i = partial_derivatives::partial_deepex(
            var_idx,
            self.deepex.ok_or(ExParseError {
                msg: "need deep expression for derivation, not possible after calling `clear`"
                    .to_string(),
            })?,
            &ops,
        )?;
        Ok(flatten(d_i))
    }
    fn unparse(&self) -> Result<String, ExParseError> {
        match &self.deepex {
            Some(deepex) => Ok(deepex.unparse()),
            None => Err(ExParseError {
                msg: "unparse impossible, since deep expression optimized away".to_string(),
            }),
        }
    }
    fn reduce_memory(&mut self) {
        self.deepex = None;
    }
}

/// The expression is displayed as a string created by [`unparse`](FlatEx::unparse).
impl<'a, T: Copy + Debug> Display for FlatEx<'a, T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let unparsed = self.unparse();
        match unparsed {
            Err(e) => write!(f, "{}", e.msg),
            Ok(s) => write!(f, "{}", s),
        }
    }
}

/// This is another representation of a flattened expression besides [`FlatEx`](FlatEx).
/// The interface is identical. The difference is that [`OwnedFlatEx`](OwnedFlatEx) can be used without
/// a lifetime parameter. All the data that [`FlatEx`](FlatEx) borrowed is kept in a
/// buffer by [`OwnedFlatEx`](OwnedFlatEx). The drawback is that parsing takes longer, since
/// additional allocations are necessary. Evaluation time should be about the same for
/// [`FlatEx`](FlatEx) and [`OwnedFlatEx`](OwnedFlatEx). To create an instance of
/// [`OwnedFlatEx`](OwnedFlatEx) you first create a [`FlatEx`](FlatEx) with one of the
/// parsing functions. In a second step you can convert this via
/// [`from_flatex`](OwnedFlatEx::from_flatex) as shown in the following.
///
/// ```rust
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// #
/// use exmex::{Expression, OwnedFlatEx};
/// let to_be_parsed = "log(z) + 2* (-z^(x-2) + sin(4*y))";
/// let expr_owned = OwnedFlatEx::<f64>::from_str(to_be_parsed)?;
/// assert!((expr_owned.eval(&[4.0, 3.7, 2.5])? - 14.992794866624788 as f64).abs() < 1e-12);
/// #
/// #     Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct OwnedFlatEx<T: Copy + Debug> {
    deepex_buf: Option<DeepBuf<T>>,
    nodes: FlatNodeVec<T>,
    ops: FlatOpVec<T>,
    prio_indices: ExprIdxVec,
    n_unique_vars: usize,
}
impl<T: Copy + Debug> OwnedFlatEx<T> {
    /// Creates an `OwnedFlatEx` instance from an instance of `FlatEx`.
    pub fn from_flatex<'a>(flatex: FlatEx<'a, T>) -> Self {
        Self {
            deepex_buf: flatex.deepex.map(|d| DeepBuf::from_deepex(&d)),
            nodes: flatex.nodes,
            ops: flatex.ops,
            prio_indices: flatex.prio_indices,
            n_unique_vars: flatex.n_unique_vars,
        }
    }
}
impl<'a, T: Copy + Debug> Expression<'a, T> for OwnedFlatEx<T> {

    /// Parses a string into an expression that can be evaluated using default operators.
    ///
    /// # Errors
    ///
    /// An error is returned in case the text cannot be parsed.
    ///
    fn from_str(text: &'a str) -> Result<Self, ExParseError>
    where
        <T as std::str::FromStr>::Err: Debug,
        T: Float + FromStr,
    {
        Ok(Self::from_flatex(FlatEx::from_str(text)?))
    }

    fn from_ops(text: &'a str, ops: &[Operator<'a, T>]) -> Result<Self, ExParseError>
    where
        <T as std::str::FromStr>::Err: Debug,
        T: Copy + FromStr + Debug,
    {
        Ok(Self::from_flatex(FlatEx::from_ops(text, ops)?))
    }

    fn from_pattern(
        text: &'a str,
        ops: &[Operator<'a, T>],
        number_regex_pattern: &str,
    ) -> Result<Self, ExParseError>
    where
        <T as std::str::FromStr>::Err: Debug,
        T: Copy + FromStr + Debug,
    {
        Ok(Self::from_flatex(FlatEx::from_pattern(
            text,
            ops,
            number_regex_pattern,
        )?))
    }

    fn eval(&self, vars: &[T]) -> Result<T, ExParseError> {
        flat_details::eval_flatex(
            vars,
            &self.nodes,
            &self.ops,
            &self.prio_indices,
            self.n_unique_vars,
        )
    }

    fn partial(self, var_idx: usize) -> Result<Self, ExParseError>
    where
        T: Float,
    {
        let ops = operators::make_default_operators();
        let deep_buf = match self.deepex_buf {
            Some(d) => Ok(d),
            None => Err(ExParseError {
                msg: "need deep expression for derivation, not possible after calling `clear`"
                    .to_string(),
            }),
        }?;
        let deepex = deep_buf.to_deepex(&ops)?;
        let d_i = partial_derivatives::partial_deepex(var_idx, deepex, &ops)?;
        Ok(Self::from_flatex(flatten(d_i)))
    }

    fn unparse(&self) -> Result<String, ExParseError> {
        match &self.deepex_buf {
            Some(deepex) => Ok(deepex.unparsed.clone()),
            None => Err(ExParseError {
                msg: "unparse impossible, since deep expression optimized away".to_string(),
            }),
        }
    }

    fn reduce_memory(&mut self) {
        self.deepex_buf = None;
    }
}
/// The expression is displayed as a string created by [`unparse`](OwnedFlatEx::unparse).
impl<T: Copy + Debug> Display for OwnedFlatEx<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let unparsed = self.unparse();
        match unparsed {
            Err(e) => write!(f, "{}", e.msg),
            Ok(s) => write!(f, "{}", s),
        }
    }
}

#[cfg(test)]
use crate::{
    expression::{deep::UnaryOpWithReprs, flat_details::FlatNodeKind},
    operators::{UnaryOp, VecOfUnaryFuncs},
    util::assert_float_eq_f64,
};

use super::Expression;

#[test]
fn test_operate_unary() {
    let lstr = "x+y+x+z*(-y)+x+y+x+z*(-y)+x+y+x+z*(-y)+x+y+x+z*(-y)+x+y+x+z*(-y)+x+y+x+z*(-y)+x+y+x+z*(-y)+x+y+x+z*(-y)";
    let deepex = DeepEx::<f64>::from_str(lstr).unwrap();
    let mut funcs = VecOfUnaryFuncs::new();
    funcs.push(|x: f64| x * 1.23456);
    let deepex = deepex.operate_unary(UnaryOpWithReprs {
        reprs: vec!["eagle"],
        op: UnaryOp::from_vec(funcs),
    });
    let flatex = flatten(deepex);
    assert_float_eq_f64(
        flatex.eval(&[1.0, 1.75, 2.25]).unwrap(),
        -0.23148000000000002 * 8.0,
    );
}

#[test]
fn test_flat_clear() {
    let mut flatex = FlatEx::<f64>::from_str("x*(2*(2*(2*4*8)))").unwrap();
    assert!(flatex.deepex.is_some());
    flatex.reduce_memory();
    assert!(flatex.deepex.is_none());
    assert_float_eq_f64(flatex.eval(&[1.0]).unwrap(), 2.0 * 2.0 * 2.0 * 4.0 * 8.0);
    assert_eq!(flatex.nodes.len(), 2);
    let mut flatex = OwnedFlatEx::<f64>::from_str("x*(2*(2*(2*4*8)))").unwrap();
    assert!(flatex.deepex_buf.is_some());
    flatex.reduce_memory();
    assert!(flatex.deepex_buf.is_none());
    assert_float_eq_f64(flatex.eval(&[1.0]).unwrap(), 2.0 * 2.0 * 2.0 * 4.0 * 8.0);
    assert_eq!(flatex.nodes.len(), 2);
}
#[test]
fn test_flat_compile() {
    let flatex = FlatEx::<f64>::from_str("1*sin(2-0.1)").unwrap();
    assert_float_eq_f64(flatex.eval(&[]).unwrap(), 1.9f64.sin());
    assert_eq!(flatex.nodes.len(), 1);

    let flatex = FlatEx::<f64>::from_str("x*(2*(2*(2*4*8)))").unwrap();
    assert_float_eq_f64(flatex.eval(&[1.0]).unwrap(), 2.0 * 2.0 * 2.0 * 4.0 * 8.0);
    assert_eq!(flatex.nodes.len(), 2);

    let flatex = FlatEx::<f64>::from_str("1*sin(2-0.1) + x").unwrap();
    assert_float_eq_f64(flatex.eval(&[0.0]).unwrap(), 1.9f64.sin());
    assert_eq!(flatex.nodes.len(), 2);
    match flatex.nodes[0].kind {
        FlatNodeKind::Num(n) => assert_float_eq_f64(n, 1.9f64.sin()),
        _ => assert!(false),
    }
    match flatex.nodes[1].kind {
        FlatNodeKind::Var(idx) => assert_eq!(idx, 0),
        _ => assert!(false),
    }

    let flatex = OwnedFlatEx::<f64>::from_str("y + 1 - cos(1/(1*sin(2-0.1))-2) + 2 + x").unwrap();
    assert_eq!(flatex.nodes.len(), 3);
    match flatex.nodes[0].kind {
        FlatNodeKind::Var(idx) => assert_eq!(idx, 1),
        _ => assert!(false),
    }
    match flatex.nodes[1].kind {
        FlatNodeKind::Num(_) => (),
        _ => assert!(false),
    }
    match flatex.nodes[2].kind {
        FlatNodeKind::Var(idx) => assert_eq!(idx, 0),
        _ => assert!(false),
    }
}

#[test]
fn test_display() {
    let mut flatex = flatten(DeepEx::<f64>::from_str("sin(var)/5").unwrap());
    println!("{}", flatex);
    assert_eq!(format!("{}", flatex), "sin({var})/5.0");
    flatex.reduce_memory();
    assert_eq!(
        format!("{}", flatex),
        "unparse impossible, since deep expression optimized away"
    );

    let flatex = flatten(DeepEx::<f64>::from_str("sin(var)/5").unwrap());
    let mut owned_flatex = OwnedFlatEx::from_flatex(flatex);
    assert_eq!(format!("{}", owned_flatex), "sin({var})/5.0");
    owned_flatex.reduce_memory();
    assert_eq!(
        format!("{}", owned_flatex),
        "unparse impossible, since deep expression optimized away"
    );
}

#[test]
fn test_unparse() {
    fn test(text: &str, text_ref: &str) {
        let flatex = flatten(DeepEx::<f64>::from_str(text).unwrap());
        let deepex = flatex.deepex.unwrap();
        assert_eq!(deepex.unparse(), text_ref);
        let mut flatex_reparsed = flatten(DeepEx::<f64>::from_str(text).unwrap());
        assert_eq!(flatex_reparsed.unparse().unwrap(), text_ref);
        flatex_reparsed.reduce_memory();
        assert!(flatex_reparsed.unparse().is_err());
    }
    let text = "5+x";
    let text_ref = "5.0+{x}";
    test(text, text_ref);
    let text = "sin(5+var)^(1/{y})+{var}";
    let text_ref = "sin(5.0+{var})^(1.0/{y})+{var}";
    test(text, text_ref);
    let text = "-(5+var)^(1/{y})+{var}";
    let text_ref = "-(5.0+{var})^(1.0/{y})+{var}";
    test(text, text_ref);
    let text = "cos(sin(-(5+var)^(1/{y})))+{var}";
    let text_ref = "cos(sin(-(5.0+{var})^(1.0/{y})))+{var}";
    test(text, text_ref);
    let text = "cos(sin(-5+var^(1/{y})))-{var}";
    let text_ref = "cos(sin(-5.0+{var}^(1.0/{y})))-{var}";
    test(text, text_ref);
    let text = "cos(sin(-z+var*(1/{y})))+{var}";
    let text_ref = "cos(sin(-({z})+{var}*(1.0/{y})))+{var}";
    test(text, text_ref);
}
