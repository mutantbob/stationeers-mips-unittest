use super::*;

#[test]
pub fn test_bdns() -> Result<(),MultiError>
{
    let source = include_str!("tests/test_bdns.mips");

    let program = compile(source)?;

    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 2.0);
    }
    {
        let mut ctx = CPUContext::new_simple(&program);
        ctx.attach_device(0, DeviceState::new())?;
        ctx = execute_until_yields(&program, ctx, 99)?;
        assert_eq!(ctx.register_reference(Register{idx:0})?, 3.0);
    }
    Ok(())
}