use super::*;

/*
This is a (probably-incomplete) test of the Stationeers/MIPS instructions listed at
https://stationeering.com/tools/ic
Some of that documentation is mangled (probably by improper HTML escaping of < and > characters)
*/


/// bdns: branch if device not set
#[test]
pub fn test_bdns() -> Result<(),MultiError>
{
    let source = include_str!("tests/test_bdns.mips");

    let program = compile(source)?;

    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 2.0);
        assert!(ctx.get_ra().is_nan());
    }
    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx.attach_device(0, DeviceState::new())?;
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 3.0);
    }
    Ok(())
}

// bdnsal: branch if device not set and link
#[test]
pub fn test_bdnsal() -> Result<(),MultiError>
{
    let source = include_str!("tests/test_bdnsal.mips");

    let program = compile(source)?;

    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 2.0);
        assert_eq!(ctx.get_ra(), 2.0);
    }
    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx.attach_device(0, DeviceState::new())?;
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 3.0);
        assert!(ctx.get_ra().is_nan());
    }
    Ok(())
}

/// bdse: branch if device set
#[test]
pub fn test_bdse() -> Result<(),MultiError>
{
    let source = include_str!("tests/test_bdse.mips");

    let program = compile(source)?;

    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 4.0);
        assert_eq!(ctx.register_reference(Register{idx:1})?, 11.0);
        assert!(ctx.get_ra().is_nan());
    }
    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx.attach_device(0, DeviceState::new())?;
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 5.0);
        assert_eq!(ctx.register_reference(Register{idx:1})?, 11.0);
    }
    Ok(())
}

// bdseal: branch if device set and link
#[test]
pub fn test_bdseal() -> Result<(),MultiError>
{
    let source = include_str!("tests/test_bdseal.mips");

    let program = compile(source)?;

    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 4.0);
        assert!(ctx.get_ra().is_nan());
    }
    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx.attach_device(0, DeviceState::new())?;
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 5.0);
        assert_eq!(ctx.get_ra(), 2.0);
    }
    Ok(())
}

// brdns: branch relative if device not set
#[test]
pub fn test_brdns() -> Result<(),MultiError>
{
    let source = include_str!("tests/test_brdns.mips");

    let program = compile(source)?;

    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 2.0);
        assert!(ctx.register_reference(Register{idx:1})?.is_nan());
        assert!(ctx.get_ra().is_nan());
    }
    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx.attach_device(0, DeviceState::new())?;
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 3.0);
        assert!(ctx.get_ra().is_nan());
    }
    Ok(())
}

// brdse: branch relative if device set
#[test]
pub fn test_brdse() -> Result<(),MultiError>
{
    let source = include_str!("tests/test_brdse.mips");

    let program = compile(source)?;

    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 4.0);
        assert!(ctx.get_ra().is_nan());
    }
    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx.attach_device(0, DeviceState::new())?;
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 5.0);
        assert!(ctx.register_reference(Register{idx:1})?.is_nan());
        assert!(ctx.get_ra().is_nan());
    }
    Ok(())
}

//

#[test]
pub fn test_load1() -> Result<(),MultiError>
{
    let source = include_str!("tests/test_l.mips");
    let program = compile(source)?;

    let ctx = CPUContext::new_simple(&program);
    assert!( execute_until_yields(&program, ctx, 99).is_err() , "should have failed");

    Ok(())
}

#[test]
pub fn test_load2() -> Result<(),MultiError>
{
    let source = include_str!("tests/test_l.mips");
    let program = compile(source)?;

    let mut ctx = CPUContext::new_simple(&program);
    ctx.attach_device(0, DeviceStateBuilder::new().set("Bacon", 7.5).build())?;
    ctx = execute_until_yields(&program, ctx, 99)?;
    assert_eq!(ctx.register_reference(Register{idx:0})?, 7.5);

    Ok(())
}

#[test]
pub fn test_load3() -> Result<(),MultiError>
{
    let source = include_str!("tests/test_l.mips");
    let program = compile(source)?;

    let mut ctx = CPUContext::new_simple(&program);
    ctx.attach_device(0, DeviceState::new())?;
    ctx = execute_until_yields(&program, ctx, 99)?;
    // XXX does the game really return 0 for fields that do not exist on a device?
    assert_eq!(ctx.register_reference(Register{idx:0})?, 0.0);

    Ok(())
}

//

#[test]
pub fn test_store1() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_s.mips");
    let program = compile(source)?;

    let ctx = CPUContext::new_simple(&program);
    assert!( execute_until_yields(&program, ctx, 99).is_err(), "should have died on unlinked device");

    Ok(())
}

#[test]
pub fn test_store2() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_s.mips");
    let program = compile(source)?;

    let mut ctx = CPUContext::new_simple(&program);
    ctx.attach_device(0, DeviceState::new())?;
    ctx = execute_until_yields(&program, ctx, 99)?;

    let dev_state = ctx.device_reference(Device::Regular(0))?;
    assert_eq!( *dev_state.get("Nyan").unwrap(), 9000_f32);
    assert_eq!( *dev_state.get("Cake").unwrap(), 5.0);
    assert_eq!( *dev_state.get("Price").unwrap(), 4.75);

    Ok(())
}

//

#[test]
pub fn test_bap1() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_bap.mips");
    let program = compile(source)?;

    let mut ctx = CPUContext::new_simple(&program);
    *ctx.register_reference_mut(Register{idx:0})? = 4.0;
    *ctx.register_reference_mut(Register{idx:1})? = 4.01;
    *ctx.register_reference_mut(Register{idx:2})? = 0.01;
    ctx = execute_until_yields(&program, ctx, 99)?;

    assert_eq!(ctx.register_reference(Register{idx:9})?, 1.0);

    Ok(())
}

#[test]
pub fn test_bap2() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_bap.mips");
    let program = compile(source)?;

    let mut ctx = CPUContext::new_simple(&program);
    *ctx.register_reference_mut(Register{idx:0})? = 4.0;
    *ctx.register_reference_mut(Register{idx:1})? = 4.05;
    *ctx.register_reference_mut(Register{idx:2})? = 0.01;
    ctx = execute_until_yields(&program, ctx, 99)?;

    assert_eq!(ctx.register_reference(Register{idx:9})?, 2.5);

    Ok(())
}

#[test]
pub fn test_bap3() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_bap.mips");
    let program = compile(source)?;

    let mut ctx = CPUContext::new_simple(&program);
    *ctx.register_reference_mut(Register{idx:0})? = -4.0;
    *ctx.register_reference_mut(Register{idx:1})? = -4.01;
    *ctx.register_reference_mut(Register{idx:2})? = 0.01;
    ctx = execute_until_yields(&program, ctx, 99)?;

    assert_eq!(ctx.register_reference(Register{idx:9})?, 1.0);

    Ok(())
}

//

#[test]
pub fn test_bapal1() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_bapal.mips");
    let program = compile(source)?;

    let mut ctx = CPUContext::new_simple(&program);
    *ctx.register_reference_mut(Register{idx:0})? = 4.0;
    *ctx.register_reference_mut(Register{idx:1})? = 4.01;
    *ctx.register_reference_mut(Register{idx:2})? = 0.01;
    ctx = execute_until_yields(&program, ctx, 99)?;

    assert_eq!(ctx.register_reference(Register{idx:9})?, -4.0);
    assert_eq!(ctx.get_ra(), 1.0);

    Ok(())
}

#[test]
pub fn test_bapal2() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_bapal.mips");
    let program = compile(source)?;

    let mut ctx = CPUContext::new_simple(&program);
    *ctx.register_reference_mut(Register{idx:0})? = 4.0;
    *ctx.register_reference_mut(Register{idx:1})? = 4.05;
    *ctx.register_reference_mut(Register{idx:2})? = 0.01;
    ctx = execute_until_yields(&program, ctx, 99)?;

    assert_eq!(ctx.register_reference(Register{idx:9})?, 3.5);
    assert!(ctx.get_ra().is_nan());

    Ok(())
}

#[test]
pub fn test_bapal3() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_bapal.mips");
    let program = compile(source)?;

    let mut ctx = CPUContext::new_simple(&program);
    *ctx.register_reference_mut(Register{idx:0})? = -3.0;
    *ctx.register_reference_mut(Register{idx:1})? = -3.01;
    *ctx.register_reference_mut(Register{idx:2})? = 0.01;
    ctx = execute_until_yields(&program, ctx, 99)?;

    assert_eq!(ctx.register_reference(Register{idx:9})?, -4.0);
    assert_eq!(ctx.get_ra(), 1.0);

    Ok(())
}

//

/// check that the program when run with r0=a; r1=b; finishes with r9==expected
pub fn check_binary_operator_019(program:&CompiledProgram, a:f32, b:f32, expected:f32) ->Result<(), MultiError>
{

    let mut ctx = CPUContext::new_simple(&program);
    *ctx.register_reference_mut(Register{idx:0})? = a;
    *ctx.register_reference_mut(Register{idx:1})? = b;
    ctx = execute_until_yields(&program, ctx, 99)?;

    assert_eq!(ctx.register_reference(Register{idx:9})?, expected);

    Ok(())
}

/// check that the program when run with r0=a; finishes with r9==expected
pub fn check_unary_operator_09(program:&CompiledProgram, a:f32, expected:f32) ->Result<(), MultiError>
{

    let mut ctx = CPUContext::new_simple(&program);
    *ctx.register_reference_mut(Register{idx:0})? = a;
    ctx = execute_until_yields(&program, ctx, 99)?;

    assert_eq!(ctx.register_reference(Register{idx:9})?, expected);

    Ok(())
}

//

#[test]
pub fn test_beq1() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_beq.mips");
    let program = compile(source)?;

    check_binary_operator_019(&program, -3.0, -3.01, 7.0)
}

#[test]
pub fn test_beq2() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_beq.mips");
    let program = compile(source)?;

    check_binary_operator_019(&program, -3.0, -3.0, 42.0)
}

//

#[test]
pub fn test_beqal1() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_beqal.mips");
    let program = compile(source)?;

    check_binary_operator_019(&program, 2.0, -3.0, 7.0)
}

#[test]
pub fn test_beqal2() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_beqal.mips");
    let program = compile(source)?;

    check_binary_operator_019(&program, -3.0, -3.0, 42.0)
}

#[test]
pub fn test_beqal3() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_beqal.mips");
    let program = compile(source)?;

    check_binary_operator_019(&program, 2.0, 5.0, 7.0)
}

//

#[test]
pub fn test_abs() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_abs.mips");
    let program = compile(source)?;

    check_unary_operator_09(&program, 2.0, 2.0)?;

    check_unary_operator_09(&program, -1.3, 1.3)
}

#[test]
pub fn test_ceil() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_ceil.mips");
    let program = compile(source)?;

    check_unary_operator_09(&program, 2.0, 2.0)?;
    check_unary_operator_09(&program, 2.1, 3.0)?;
    check_unary_operator_09(&program, -1.3, -1.)?;
    check_unary_operator_09(&program, -7., -7.)
}

#[test]
pub fn test_floor() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_floor.mips");
    let program = compile(source)?;

    check_unary_operator_09(&program, 2.0, 2.0)?;
    check_unary_operator_09(&program, 2.1, 2.0)?;
    check_unary_operator_09(&program, -1.3, -2.)?;
    check_unary_operator_09(&program, -7., -7.)
}

#[test]
pub fn test_log() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_log.mips");
    let program = compile(source)?;

    check_unary_operator_09(&program, 2.0, 2.0_f32.ln())
}

#[test]
pub fn test_round() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_round.mips");
    let program = compile(source)?;

    check_unary_operator_09(&program, 2.5, 2.0)?;
    check_unary_operator_09(&program, 3.5, 4.0)?;
    check_unary_operator_09(&program, 3.4, 3.0)
}

#[test]
pub fn test_sqrt() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_sqrt.mips");
    let program = compile(source)?;

    check_unary_operator_09(&program, 6.25, 2.5)?;
    check_unary_operator_09(&program, 49., 7.0)
}

//

#[test]
pub fn test_add() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_add.mips");
    let program = compile(source)?;

    check_binary_operator_019(&program, 2.0, 2.5, 4.5)?;

    check_binary_operator_019(&program, 3.0, -1.0, 2.0)
}

#[test]
pub fn test_exp() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_exp.mips");
    let program = compile(source)?;

    check_unary_operator_09(&program, 2.0, 2.0_f32.exp())
}

#[test]
pub fn test_div() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_div.mips");
    let program = compile(source)?;

    check_binary_operator_019(&program, 7.5, 2.5, 3.0)?;

    check_binary_operator_019(&program, 3.0, -2.0, -1.5)
}

#[test]
pub fn test_mul() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_mul.mips");
    let program = compile(source)?;

    check_binary_operator_019(&program, 2.0, 2.5, 5.0)?;

    check_binary_operator_019(&program, 3.0, -1.0, -3.0)
}

#[test]
pub fn test_max() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_max.mips");
    let program = compile(source)?;

    check_binary_operator_019(&program, 2.0, 2.5, 2.5)?;

    check_binary_operator_019(&program, 3.0, -1.0, 3.0)
}

#[test]
pub fn test_min() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_min.mips");
    let program = compile(source)?;

    check_binary_operator_019(&program, 2.0, 2.5, 2.0)?;

    check_binary_operator_019(&program, 3.0, -1.0, -1.0)
}

#[test]
pub fn test_mod() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_mod.mips");
    let program = compile(source)?;

    check_binary_operator_019(&program, 2.0, 2.5, 2.0)?;
    check_binary_operator_019(&program, 7.1, 2.5, 2.1)?;
    check_binary_operator_019(&program, 3.25, 1.25, 0.75)
}

#[test]
pub fn test_rand() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_rand.mips");
    let program = compile(source)?;

    for _i in 0..10 {

        let mut ctx = CPUContext::new_simple(&program);
        ctx = execute_until_yields(&program, ctx, 99)?;

        let val = ctx.register_reference(Register{idx:0})?;
        let good = 0.0 <= val && val<1.0;
        assert!(good, format!("random number {} outside acceptable range [0..1)", val));

    }

    Ok(())
}

#[test]
pub fn test_sub() -> Result<(), MultiError>
{
    let source = include_str!("tests/test_sub.mips");
    let program = compile(source)?;

    check_binary_operator_019(&program, 2.0, 2.5, -0.5)?;
    check_binary_operator_019(&program, 2.1, 7.5, 2.1-7.5)
}

//

#[test]
pub fn bad_register() -> Result<(), MultiError>
{

    let source = include_str!("tests/bad_register.mips");
    let program = compile(source)?;

    {
        let ctx = CPUContext::new_simple(&program);
        assert!( execute_until_yields(&program, ctx, 99).is_err(), "should have failed to execute");
    }


    Ok(())
}

//assert!( compile(source).is_err(), "should have failed to compile");
