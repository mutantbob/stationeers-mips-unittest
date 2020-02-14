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

#[test]
pub fn test_beq1() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_beq.mips");
    let program = compile(source)?;

    let mut ctx = CPUContext::new_simple(&program);
    *ctx.register_reference_mut(Register{idx:0})? = -3.0;
    *ctx.register_reference_mut(Register{idx:1})? = -3.01;
    ctx = execute_until_yields(&program, ctx, 99)?;

    assert_eq!(ctx.register_reference(Register{idx:9})?, 7.0);

    Ok(())
}

#[test]
pub fn test_beq2() ->Result<(), MultiError>
{
    let source = include_str!("tests/test_beq.mips");
    let program = compile(source)?;

    let mut ctx = CPUContext::new_simple(&program);
    *ctx.register_reference_mut(Register{idx:0})? = -3.0;
    *ctx.register_reference_mut(Register{idx:1})? = -3.0;
    ctx = execute_until_yields(&program, ctx, 99)?;

    assert_eq!(ctx.register_reference(Register{idx:9})?, 42.0);

    Ok(())
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
