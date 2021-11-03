use std::{fmt::Debug, marker::PhantomData, str::FromStr};

use num::{Float, PrimInt};
use smallvec::SmallVec;

use crate::{data_type::DataType, BinOp, ExError, ExResult, MakeOperators, Operator};

const ARRAY_LEN: usize = 8usize;
pub type Tuple<I, F> = SmallVec<[Scalar<I, F>; ARRAY_LEN]>;

#[macro_export]
macro_rules! to_type {
    ($name:ident, $T:ty, $variant:ident) => {
        pub fn $name(self) -> ExResult<$T> {
            match self {
                Scalar::$variant(x) => Ok(x),
                _ => Err(ExError {
                    msg: format!(
                        "Scalar {:?} does not contain type {}",
                        self,
                        stringify!($variant)
                    ),
                }),
            }
        }
    };
}

#[macro_export]
macro_rules! to_scalar_type {
    ($name:ident, $T:ty) => {
        pub fn $name(self) -> ExResult<$T> {
            match self {
                Val::Scalar(s) => s.$name(),
                Val::Error(e) => Err(e),
                _ => Err(ExError {
                    msg: format!("{:?} does not contain Scalar", self),
                }),
            }
        }
    };
}

#[macro_export]
macro_rules! to_tuple_type {
    ($name:ident, $scalar_name:ident, $T:ty) => {
        pub fn $name(self) -> ExResult<SmallVec<[$T; ARRAY_LEN]>> {
            match self {
                Val::Tuple(a) => a
                    .iter()
                    .map(|s| -> ExResult<$T> { s.clone().$scalar_name() })
                    .collect::<ExResult<_>>(),
                Val::Error(e) => Err(e),
                _ => Err(ExError {
                    msg: format!("{:?} does not contain Array", self),
                }),
            }
        }
    };
}

#[macro_export]
macro_rules! from_type {
    ($name:ident, $scalar_variant:ident, $T:ty) => {
        fn $name(x: $T) -> Val<I, F> {
            Val::<I, F>::Scalar(Scalar::$scalar_variant(x))
        }
    };
}

#[derive(Clone, Debug)]
pub enum Scalar<I: DataType + PrimInt, F: DataType + Float> {
    Int(I),
    Float(F),
    Bool(bool),
}
impl<I, F> Scalar<I, F>
where
    I: DataType + PrimInt,
    F: DataType + Float,
{
    to_type!(to_float, F, Float);
    to_type!(to_int, I, Int);
    to_type!(to_bool, bool, Bool);
}
#[derive(Clone, Debug)]
pub enum Val<I = i32, F = f64>
where
    I: DataType + PrimInt,
    F: DataType + Float,
{
    Scalar(Scalar<I, F>),
    Tuple(Tuple<I, F>),
    // since the trait `Try` is experimental, we keep track of an error in an additional arm
    Error(ExError),
}

impl<I, F> Val<I, F>
where
    I: DataType + PrimInt,
    F: DataType + Float,
{
    to_scalar_type!(to_int, I);
    to_scalar_type!(to_float, F);
    to_scalar_type!(to_bool, bool);
    to_tuple_type!(to_int_array, to_int, I);
    to_tuple_type!(to_float_array, to_float, F);
    to_tuple_type!(to_bool_array, to_bool, bool);

    from_type!(from_float, Float, F);
    from_type!(from_int, Int, I);
    from_type!(from_bool, Bool, bool);
}

fn map_parse_err<E: Debug>(e: E) -> ExError {
    ExError {
        msg: format!("{:?}", e),
    }
}

fn parse_scalar<I, F>(s: &str) -> ExResult<Scalar<I, F>>
where
    I: DataType + PrimInt,
    F: DataType + Float,
    <I as FromStr>::Err: Debug,
    <F as FromStr>::Err: Debug,
{
    let res = Ok(if s.contains(".") {
        Scalar::Float(s.parse::<F>().map_err(map_parse_err)?)
    } else if s == "false" || s == "true" {
        Scalar::Bool(s.parse::<bool>().map_err(map_parse_err)?)
    } else {
        Scalar::Int(s.parse::<I>().map_err(map_parse_err)?)
    });
    match res {
        Result::Ok(_) => res,
        Result::Err(e) => Err(ExError {
            msg: format!("could not parse {}, {:?}", s, e),
        }),
    }
}

impl<I, F> FromStr for Val<I, F>
where
    I: DataType + PrimInt,
    F: DataType + Float,
    <I as FromStr>::Err: Debug,
    <F as FromStr>::Err: Debug,
{
    type Err = ExError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let first = s.chars().next();
        println!("parsing {}", s);
        if first == Some('[') {
            let a = &s[1..s.len() - 1];
            Ok(Val::Tuple(
                a.split(",")
                    .map(|sc_str| -> ExResult<Scalar<I, F>> { parse_scalar(sc_str.trim()) })
                    .collect::<ExResult<Tuple<I, F>>>()?,
            ))
        } else {
            Ok(Val::Scalar(parse_scalar(s)?))
        }
    }
}

/// Factory of default operators for floating point values.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ValOpsFactory<I = i32, F = f64>
where
    I: DataType + PrimInt,
    F: DataType + Float,
{
    dummy_i: PhantomData<I>,
    dummy_f: PhantomData<F>,
}

fn unpack<I, F>(v: ExResult<Val<I, F>>) -> Val<I, F>
where
    I: DataType + PrimInt,
    F: DataType + Float,
{
    match v {
        Err(e) => Val::<I, F>::Error(e),
        Ok(v) => v,
    }
}

fn pow_scalar<I, F>(a: Scalar<I, F>, b: Scalar<I, F>) -> Val<I, F>
where
    I: DataType + PrimInt,
    F: DataType + Float,
{
    match a {
        Scalar::Float(xa) => match b {
            Scalar::Float(xb) => Val::<I, F>::from_float(xa.powf(xb)),
            Scalar::Int(nb) => {
                let powered = xa.powi(nb.to_i32().unwrap());
                Val::<I, F>::from_float(powered)
            }
            Scalar::Bool(_) => Val::Error(ExError::from_str("cannot use bool as exponent")),
        },
        Scalar::Int(na) => match b {
            Scalar::Float(_) => Val::Error(ExError::from_str("cannot use float as exponent")),
            Scalar::Int(nb) => {
                let powered = na.pow(nb.to_u32().unwrap());

                Val::<I, F>::from_int(powered)
            }
            Scalar::Bool(_) => Val::Error(ExError::from_str("cannot use bool as exponent")),
        },
        Scalar::Bool(_) => Val::Error(ExError::from_str("cannot use bool as base")),
    }
}

fn pow<I, F>(base: Val<I, F>, exponent: Val<I, F>) -> Val<I, F>
where
    I: DataType + PrimInt,
    F: DataType + Float,
{
    match base {
        Val::Scalar(x) => match exponent {
            Val::Scalar(y) => pow_scalar(x, y),
            Val::Tuple(_) => Val::Error(ExError::from_str("cannot use array as exponent")),
            Val::Error(e) => Val::Error(e),
        },
        Val::Tuple(aa) => match exponent {
            Val::Scalar(y) => {
                let powered = aa
                    .iter()
                    .map(|xi| match pow_scalar(xi.clone(), y.clone()) {
                        Val::Scalar(s) => Ok(s),
                        Val::Tuple(_) => Err(ExError::from_str("we only allow tuples of scalars")),
                        Val::Error(e) => Err(ExError::from_str(e.msg.as_str())),
                    })
                    .collect::<ExResult<Tuple<I, F>>>();
                match powered {
                    Err(e) => Val::Error(e),
                    Ok(a) => Val::Tuple(a),
                }
            }
            Val::Tuple(_) => Val::Error(ExError::from_str("cannot use tuple as exponent")),
            Val::Error(e) => Val::Error(e),
        },
        Val::Error(e) => Val::Error(e),
    }
}

#[macro_export]
macro_rules! base_arith_scalar {
    ($name:ident, $op:ident) => {
        fn $name<I, F>(a: Scalar<I, F>, b: Scalar<I, F>) -> Val<I, F>
        where
            I: DataType + PrimInt,
            F: DataType + Float,
        {
            match a {
                Scalar::Float(xa) => match b {
                    Scalar::Float(xb) => Val::<I, F>::from_float(xa.$op(xb)),
                    _ => Val::Error(ExError::from_str(
                        format!("cannot only {} float to float", stringify!($op)).as_str(),
                    )),
                },
                Scalar::Int(na) => match b {
                    Scalar::Int(nb) => Val::<I, F>::from_int(na.$op(nb)),
                    _ => Val::Error(ExError::from_str(
                        format!("cannot only {} int to int", stringify!($op)).as_str(),
                    )),
                },
                Scalar::Bool(_) => Val::Error(ExError::from_str(
                    format!("cannot use bool in {}", stringify!($op)).as_str(),
                )),
            }
        }
    };
}

#[macro_export]
macro_rules! base_arith_scalar_to_tuple {
    ($name:ident, $op_scalar:ident) => {
        fn $name<I, F>(a: Scalar<I, F>, b: Tuple<I, F>) -> Val<I, F>
        where
            I: DataType + PrimInt,
            F: DataType + Float,
        {
            let tuple = b
                .iter()
                .map(|bi| match $op_scalar(a.clone(), bi.clone()) {
                    Val::Scalar(s) => Ok(s),
                    Val::Tuple(_) => Err(ExError::from_str("tuples of tuples are not supported")),
                    Val::Error(e) => Err(e),
                })
                .collect::<ExResult<Tuple<I, F>>>();
            match tuple {
                Ok(t) => Val::Tuple(t),
                Err(e) => Val::Error(e),
            }
        }
    };
}

#[macro_export]
macro_rules! base_arith_tuple {
    ($name:ident, $op_scalar:ident) => {
        fn $name<I, F>(a: Tuple<I, F>, b: Tuple<I, F>) -> Val<I, F>
        where
            I: DataType + PrimInt,
            F: DataType + Float,
        {
            let tuple = (0..a.len())
                .map(|i| match $op_scalar(a[i].clone(), b[i].clone()) {
                    Val::Scalar(s) => Ok(s),
                    Val::Tuple(_) => Err(ExError::from_str("tuples of tuples are not supported")),
                    Val::Error(e) => Err(e),
                })
                .collect::<ExResult<Tuple<I, F>>>();
            match tuple {
                Ok(t) => Val::Tuple(t),
                Err(e) => Val::Error(e),
            }
        }
    };
}

#[macro_export]
macro_rules! base_arith {
    ($name:ident, $op_scalar:ident, $op_scalar_to_tuple:ident, $op_tuple:ident) => {
        fn $name<I, F>(a: Val<I, F>, b: Val<I, F>) -> Val<I, F>
        where
            I: DataType + PrimInt,
            F: DataType + Float,
        {
            match a {
                Val::Scalar(x) => match b {
                    Val::Scalar(y) => $op_scalar(x, y),
                    Val::Tuple(b) => $op_scalar_to_tuple(x, b),
                    Val::Error(e) => Val::Error(e),
                },
                Val::Tuple(t) => match b {
                    Val::Scalar(y) => $op_scalar_to_tuple(y, t),

                    Val::Tuple(t2) => $op_tuple(t, t2),
                    Val::Error(e) => Val::Error(e),
                },
                Val::Error(e) => Val::Error(e),
            }
        }
    };
}

base_arith_scalar!(add_scalar, add);
base_arith_scalar_to_tuple!(add_scalar_to_tuple, add_scalar);
base_arith_tuple!(add_tuple, add_scalar);
base_arith!(add, add_scalar, add_scalar_to_tuple, add_tuple);

base_arith_scalar!(sub_scalar, sub);
base_arith_scalar_to_tuple!(sub_scalar_to_tuple, sub_scalar);
base_arith_tuple!(sub_tuple, sub_scalar);
base_arith!(sub, sub_scalar, sub_scalar_to_tuple, sub_tuple);

base_arith_scalar!(mul_scalar, mul);
base_arith_scalar_to_tuple!(mul_scalar_to_tuple, mul_scalar);
base_arith_tuple!(mul_tuple, mul_scalar);
base_arith!(mul, mul_scalar, mul_scalar_to_tuple, mul_tuple);

base_arith_scalar!(div_scalar, div);
base_arith_scalar_to_tuple!(div_scalar_to_tuple, div_scalar);
base_arith_tuple!(div_tuple, div_scalar);
base_arith!(div, div_scalar, div_scalar_to_tuple, div_tuple);

#[macro_export]
macro_rules! fold_tuple {
    ($name:ident, $init_float:literal, $init_int:literal, $folder:expr) => {
        fn $name<I, F>(tuple: Val<I, F>) -> Val<I, F>
        where
            I: DataType + PrimInt,
            F: DataType + Float,
        {
            match tuple.clone().to_float_array() {
                Ok(t) => Val::from_float(t.iter().fold(F::from($init_float).unwrap(), $folder)),
                Err(_) => match tuple.to_int_array() {
                    Ok(t) => Val::from_int(t.iter().fold(I::from($init_int).unwrap(), $folder)),
                    Err(e) => Val::Error(e),
                },
            }
        }
    };
}

fold_tuple!(sum, 0.0, 0, |x, y| x + *y);
fold_tuple!(prod, 1.0, 1, |x, y| x * *y);

#[macro_export]
macro_rules! unary_scalar {
    ($name:ident, $to_type:ident, $from_type:ident) => {
        fn $name<I, F>(scalar: Val<I, F>) -> Val<I, F>
        where
            I: DataType + PrimInt,
            F: DataType + Float,
        {
            match scalar {
                Val::Scalar(s) => match s.$to_type() {
                    Ok(x) => Val::<I, F>::$from_type(x.$name()),
                    Err(e) => Val::Error(e),
                },
                Val::Tuple(t) => Val::Error(ExError {
                    msg: format!("expected scalar, not {:?}", t),
                }),
                Val::Error(e) => Val::Error(e),
            }
        }
    };
}

unary_scalar!(signum, to_float, from_float);
unary_scalar!(sin, to_float, from_float);
unary_scalar!(cos, to_float, from_float);
unary_scalar!(tan, to_float, from_float);
unary_scalar!(asin, to_float, from_float);
unary_scalar!(acos, to_float, from_float);
unary_scalar!(atan, to_float, from_float);
unary_scalar!(sinh, to_float, from_float);
unary_scalar!(cosh, to_float, from_float);
unary_scalar!(tanh, to_float, from_float);
unary_scalar!(floor, to_float, from_float);
unary_scalar!(ceil, to_float, from_float);
unary_scalar!(trunc, to_float, from_float);
unary_scalar!(fract, to_float, from_float);
unary_scalar!(exp, to_float, from_float);
unary_scalar!(sqrt, to_float, from_float);
unary_scalar!(ln, to_float, from_float);
unary_scalar!(log2, to_float, from_float);

impl<I, F> MakeOperators<Val<I, F>> for ValOpsFactory<I, F>
where
    I: DataType + PrimInt,
    F: DataType + Float,
    <I as FromStr>::Err: Debug,
    <F as FromStr>::Err: Debug,
{
    /// Returns the default operators.
    fn make<'a>() -> Vec<Operator<'a, Val<I, F>>> {
        vec![
            Operator::make_bin(
                "^",
                BinOp {
                    apply: |a, b| pow(a, b),
                    prio: 4,
                    is_commutative: false,
                },
            ),
            Operator::make_bin(
                "+",
                BinOp {
                    apply: |a, b| add(a, b),
                    prio: 1,
                    is_commutative: true,
                },
            ),
            Operator::make_bin(
                "-",
                BinOp {
                    apply: |a, b| sub(a, b),
                    prio: 1,
                    is_commutative: false,
                },
            ),
            Operator::make_bin(
                "*",
                BinOp {
                    apply: |a, b| mul(a, b),
                    prio: 2,
                    is_commutative: true,
                },
            ),
            Operator::make_bin(
                "/",
                BinOp {
                    apply: |a, b| div(a, b),
                    prio: 3,
                    is_commutative: false,
                },
            ),
            Operator::make_unary("sum", |a| sum(a)),
            Operator::make_unary("prod", |a| prod(a)),
            Operator::make_unary("signum", |a| signum(a)),
            Operator::make_unary("sin", |a| sin(a)),
            Operator::make_unary("cos", |a| cos(a)),
            Operator::make_unary("tan", |a| tan(a)),
            Operator::make_unary("asin", |a| asin(a)),
            Operator::make_unary("acos", |a| acos(a)),
            Operator::make_unary("atan", |a| atan(a)),
            Operator::make_unary("sinh", |a| sinh(a)),
            Operator::make_unary("cosh", |a| cosh(a)),
            Operator::make_unary("tanh", |a| tanh(a)),
            Operator::make_unary("floor", |a| floor(a)),
            Operator::make_unary("ceil", |a| ceil(a)),
            Operator::make_unary("trunc", |a| trunc(a)),
            Operator::make_unary("fract", |a| fract(a)),
            Operator::make_unary("exp", |a| exp(a)),
            Operator::make_unary("sqrt", |a| sqrt(a)),
            Operator::make_unary("log", |a| ln(a)),
            Operator::make_unary("log2", |a| log2(a)),
            Operator::make_constant("PI", Val::from_float(F::from(std::f64::consts::PI).unwrap())),
            Operator::make_constant("π", Val::from_float(F::from(std::f64::consts::PI).unwrap())),
            Operator::make_constant("E", Val::from_float(F::from(std::f64::consts::E).unwrap())),
        ]
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        util::assert_float_eq_f64,
        value::{Scalar, Val},
        ExError, ExResult, Express, FlatEx,
    };
    use smallvec::smallvec;

    use super::ValOpsFactory;
    #[test]
    fn test_to() -> ExResult<()> {
        assert_eq!(Scalar::<i32, f64>::Float(3.4).to_float()?, 3.4);
        assert_eq!(Scalar::<i32, f64>::Int(123).to_int()?, 123);
        assert!(Scalar::<i32, f64>::Bool(true).to_bool()?);
        assert!(Scalar::<i32, f64>::Bool(false).to_int().is_err());
        assert_eq!(Val::<i32, f64>::Scalar(Scalar::Float(3.4)).to_float()?, 3.4);
        assert_eq!(Val::<i32, f64>::Scalar(Scalar::Int(34)).to_int()?, 34);
        assert!(!Val::<i32, f64>::Scalar(Scalar::Bool(false)).to_bool()?);
        assert!(
            Val::<i32, f64>::Tuple(smallvec![Scalar::Int(1), Scalar::Int(1), Scalar::Int(1)])
                .to_int()
                == Err(ExError {
                    msg: "Array([Int(1), Int(1), Int(1)]) does not contain Scalar".to_string()
                })
        );
        assert_eq!(
            Val::<i32, f64>::Tuple(smallvec![Scalar::Int(1), Scalar::Int(2)]).to_int_array(),
            Ok(smallvec![1, 2])
        );
        Ok(())
    }

    #[test]
    fn test() -> ExResult<()> {
        let pattern =
            r"\.?[0-9]+(\.[0-9]+)?|true|false|\[\-?.?[0-9]+(\.[0-9]+)?(,-?\.?[0-9]+(\.[0-9]+)?)*\]";
        fn test_int(s: &str, reference: i32, pattern: &str) -> ExResult<()> {
            let expr = FlatEx::<Val, ValOpsFactory>::from_pattern(s, pattern).unwrap();
            assert_eq!(reference, expr.eval(&[])?.to_int()?);
            Ok(())
        }
        fn test_float(s: &str, reference: f64, pattern: &str) -> ExResult<()> {
            let expr = FlatEx::<Val, ValOpsFactory>::from_pattern(s, pattern).unwrap();
            assert_float_eq_f64(reference, expr.eval(&[])?.to_float()?);
            Ok(())
        }

        test_int("2^4", 16, pattern)?;
        test_int("2+4", 6, pattern)?;
        test_int("9+4", 13, pattern)?;
        test_int("9+4^2", 25, pattern)?;
        test_int("9/4", 2, pattern)?;
        test_float("2.5+4.0^2", 18.5, pattern)?;
        test_float("2.5*4.0^2", 2.5 * 4.0 * 4.0, pattern)?;
        test_float("2.5-4.0^2", -13.5, pattern)?;
        test_float("9.0/4.0", 9.0 / 4.0, pattern)?;
        test_float("sum([9.0,4.0])", 13.0, pattern)?;
        test_int("sum([9,1])", 10, pattern)?;
        test_float("sum([9.0])", 9.0, pattern)?;
        test_float("sum([9.0,3.2]+[1.0,2.0])", 15.2, pattern)?;
        test_float(
            "prod([9.0,3.2,-1.6]+[1.0,2.0,7.45])",
            10.0 * 5.2 * (7.45 - 1.6),
            pattern,
        )?;
        test_float("sin(9.0)", 9.0f64.sin(), pattern)?;
        test_float("cos(91.0)", 91.0f64.cos(), pattern)?;
        test_float("tan(913.0)", 913.0f64.tan(), pattern)?;
        test_float("sin(π)", 0.0, pattern)?;
        test_float("cos(π)", -1.0, pattern)?;

        Ok(())
    }
}
