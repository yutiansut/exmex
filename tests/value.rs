#[cfg(feature = "value")]
use {
    exmex::{ExResult, Express, Val},
    utils::assert_float_eq_f64,
};
#[cfg(feature = "value")]
mod utils;
#[test]
#[cfg(feature = "value")]
fn test_vars() -> ExResult<()> {
    let expr = exmex::parse_val::<i32, f64>("x+5.3")?;
    utils::assert_float_eq_f64(expr.eval(&[Val::Float(3.4)])?.to_float()?, 8.7);

    let expr = exmex::parse_val_owned::<i32, f64>("-(x1 if x0 else x2)+5.3")?;
    let res = expr
        .eval(&[Val::Bool(true), Val::Float(3.4), Val::Int(3)])?
        .to_float()?;

    utils::assert_float_eq_f64(res, 1.9);

    let expr = exmex::parse_val_owned::<i64, f32>("-sin(x)+5.3")?;
    utils::assert_float_eq_f32(
        expr.eval(&[Val::Float(2.2)])?.to_float()?,
        -2.2f32.sin() + 5.3,
    );

    let expr = exmex::parse_val_owned::<i64, f32>("-sin(x) if y > 0 else z + 3")?;
    utils::assert_float_eq_f32(
        expr.eval(&[Val::Float(1.0), Val::Int(2), Val::Int(3)])?
            .to_float()?,
        -1f32.sin(),
    );
    assert_eq!(
        expr.eval(&[Val::Float(1.0), Val::Int(-1), Val::Int(3)])?
            .to_int()?,
        6,
    );
    
    let expr = exmex::parse_val::<i32, f64>("z if false else 2")?;
    println!("{:#?}", expr);
    assert_eq!(
        expr.eval(&[Val::Int(-3)])?
            .to_int()?,
        2,
    );

    Ok(())
}

#[test]
#[cfg(feature = "value")]
fn test_readme() -> ExResult<()> {
    let expr = exmex::parse_val::<i32, f64>("0 if b < c else 1.2")?;
    let res = expr.eval(&[Val::Float(34.0), Val::Int(21)])?.to_float()?;
    assert!((res - 1.2).abs() < 1e-12);
    Ok(())
}

#[test]
#[cfg(feature = "serde")]
#[cfg(feature = "value")]
fn test_serde_public() -> ExResult<()> {
    use exmex::{FlatExVal, OwnedFlatExVal};

    let s = "{x}^3.0 if z < 0 else y";

    // flatex
    let flatex = FlatExVal::<i32, f64>::from_str(s)?;
    let serialized = serde_json::to_string(&flatex).unwrap();
    let deserialized = serde_json::from_str::<FlatExVal<i32, f64>>(serialized.as_str()).unwrap();
    assert_eq!(deserialized.n_vars(), 3);
    let res = deserialized.eval(&[Val::Float(2.0), Val::Bool(false), Val::Float(1.0)])?;
    assert_eq!(res.to_bool()?, false);
    let res = deserialized.eval(&[Val::Float(2.0), Val::Float(1.0), Val::Int(-1)])?;
    assert_float_eq_f64(res.to_float()?, 8.0);
    assert_eq!(s, format!("{}", deserialized));

    // owned flatex from flatex
    let flatex = OwnedFlatExVal::<i32, f64>::from_flatex(flatex);
    let serialized = serde_json::to_string(&flatex).unwrap();
    let deserialized = serde_json::from_str::<FlatExVal::<i32, f64>>(serialized.as_str()).unwrap();
    assert_eq!(deserialized.n_vars(), 3);
    let res = deserialized.eval(&[Val::Float(2.0), Val::Bool(false), Val::Float(1.0)])?;
    assert_eq!(res.to_bool()?, false);
    let res = deserialized.eval(&[Val::Float(2.0), Val::Float(1.0), Val::Int(-1)])?;
    assert_float_eq_f64(res.to_float()?, 8.0);
    assert_eq!(s, format!("{}", deserialized));

    // owned flatex from string
    let flatex = OwnedFlatExVal::<i32, f64>::from_str(s)?;
    let serialized = serde_json::to_string(&flatex).unwrap();
    let deserialized = serde_json::from_str::<FlatExVal::<i32, f64>>(serialized.as_str()).unwrap();
    assert_eq!(deserialized.n_vars(), 3);
    let res = deserialized.eval(&[Val::Float(2.0), Val::Bool(false), Val::Float(1.0)])?;
    assert_eq!(res.to_bool()?, false);
    let res = deserialized.eval(&[Val::Float(2.0), Val::Float(1.0), Val::Int(-1)])?;
    assert_float_eq_f64(res.to_float()?, 8.0);
    assert_eq!(s, format!("{}", deserialized));

    Ok(())
}
