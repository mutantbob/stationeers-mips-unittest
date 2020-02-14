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
