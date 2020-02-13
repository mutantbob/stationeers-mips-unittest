use std::collections::HashMap;

extern crate stationeers_mips_unittest;
use stationeers_mips_unittest::*;

static PROG1:&str = include_str!("prog1.mips");

static PROG2:&str = include_str!("prog2.mips");

fn main()
{
    println!("maybe you want to `cargo test`");
}

#[cfg(test)]

mod tests
{
    use super::*;

    #[test]
    fn test_prog1() ->Result<(), ExecutionError> {

        let prog1 = compile(PROG1);
        match prog1 {
            Err(err) => {
                println!("compile fail {}", err.message)
            },
            Ok(program) => {
                let mut ctx: CPUContext = CPUContext::new(program.labels(), HashMap::new(),
                                                          (0..6).map(|_| None).collect(),
                                                          (0..10).map(|_| std::f32::NAN).collect());

                ctx.attach_device(0, DeviceStateBuilder::new().set("SolarAngle", 14.0).build())?;
                ctx.attach_device(3, DeviceState::new())?;
                ctx.attach_device(4, DeviceState::new())?;

                let max_yields = 8;
                ctx = execute_until_yields(&program, ctx, max_yields)?;
                if false {
                    ctx.debug_dump();
                }
            }
        }

        Ok(())
    }

    #[test]
    fn test_prog2() ->Result<(), ExecutionError> {

        fn set_environment(ctx:&mut CPUContext, gh_pressure:f32, gh_co2:f32, pipe_pressure:f32) ->Result<(), ExecutionError>
        {
            let gh_sensor = ctx.device_reference(Device::Regular(0))?;
            gh_sensor.insert("Pressure".to_string(), gh_pressure);
            gh_sensor.insert("RatioCarbonDioxide".to_string(), gh_co2);

            let pipe_sensor = ctx.device_reference(Device::Regular(1))?;
            pipe_sensor.insert("Pressure".to_string(), pipe_pressure);

            Ok(())
        }

        fn check_pumps(ctx:&mut CPUContext, gh_on:bool, atmo_on:bool, filter_on:bool, vent_pressure:f32) ->Result<(), ExecutionError>
        {
            fn blargh(ctx:&mut CPUContext, dev_idx:u8) ->Result<f32,ExecutionError>{
                let rval = ctx.device_reference(Device::Regular(dev_idx))?.get("On").unwrap();
                println!("d{}.On = {}", dev_idx, rval);
                Ok(*rval)
            }

            assert_eq!(blargh(ctx, 3)?, if gh_on {1_f32} else {0_f32}, "wrong GH pump");
            assert_eq!(blargh(ctx, 4)?, if atmo_on {1_f32} else {0_f32}, "wrong atmo pump");
            assert_eq!(blargh(ctx, 5)?, if filter_on {1_f32} else {0_f32}, "wrong filter");

            if gh_on {
                assert_eq!(vent_pressure, ctx.get_device_field(3, "PressureExternal")?, "wrong vent pressure");
            }
            Ok(())
        }

        let prog1 = compile(PROG2);
        match prog1 {
            Err(err) => {
                println!("compile fail {}", err.message);
                assert!(false, "failed to compile");
            },
            Ok(program) => {
                let mut ctx: CPUContext = CPUContext::new(program.labels(), HashMap::new(),
                                                          (0..6).map(|_| None).collect(),
                                                          (0..10).map(|_| std::f32::NAN).collect());

                ctx.attach_device(0, DeviceState::new())?;
                ctx.attach_device(1, DeviceState::new())?;
                ctx.attach_device(3, DeviceState::new())?;
                ctx.attach_device(4, DeviceState::new())?;
                ctx.attach_device(5, DeviceState::new())?;

                {
                    set_environment(&mut ctx, 90., 0.02, 900.0)?;

                    ctx = execute_until_yields(&program, ctx, 1)?;

                    check_pumps(&mut ctx, false, true, true, 108.);
                }

                {
                    set_environment(&mut ctx, 90., 0.02, 4000.0)?;

                    ctx = execute_until_yields(&program, ctx, 1)?;

                    check_pumps(&mut ctx, false, false, true, 108.);
                }

                {
                    set_environment(&mut ctx, 125., 0.02, 900.0)?;

                    ctx = execute_until_yields(&program, ctx, 1)?;

                    check_pumps(&mut ctx, true, true, true, 108.);
                }

                {
                    set_environment(&mut ctx, 125., 0.02, 4000.0)?;

                    ctx = execute_until_yields(&program, ctx, 1)?;

                    check_pumps(&mut ctx, true, false, true, 108.);
                }

                //

                {
                    set_environment(&mut ctx, 90., 0.2, 900.0)?;

                    ctx = execute_until_yields(&program, ctx, 1)?;

                    check_pumps(&mut ctx, false, true, true, 130.);
                }

                {
                    set_environment(&mut ctx, 90., 0.2, 4000.0)?;

                    ctx = execute_until_yields(&program, ctx, 1)?;

                    check_pumps(&mut ctx, false, false, true, 130.);
                }

                {
                    set_environment(&mut ctx, 125., 0.2, 900.0)?;

                    ctx = execute_until_yields2(&program, ctx, 1,
                                                                    |ctx| {
                                                                        /*if ctx.instruction_pointer >16 && ctx.instruction_pointer < 21 {
                                                                            ctx.debug_dump();
                                                                        }*/
                                                                    }
                    )?;

                    check_pumps(&mut ctx, false, true, false, -1.);
                }

                {
                    set_environment(&mut ctx, 125., 0.2, 4000.0)?;

                    ctx = execute_until_yields(&program, ctx, 1)?;

                    check_pumps(&mut ctx, false, false, false, -1.);
                }

                //

                {
                    set_environment(&mut ctx, 132., 0.2, 4000.0)?;

                    ctx = execute_until_yields(&program, ctx, 1)?;

                    check_pumps(&mut ctx, true, false, false, 130.);
                }

                {
                    set_environment(&mut ctx, 132., 0.03, 4000.0)?;

                    ctx = execute_until_yields(&program, ctx, 1)?;

                    check_pumps(&mut ctx, true, false, true, 108.);
                }

                {
                    set_environment(&mut ctx, 132., 0.2, 200.0)?;

                    ctx = execute_until_yields(&program, ctx, 1)?;

                    check_pumps(&mut ctx, true, true, false, 130.);
                }

                {
                    set_environment(&mut ctx, 132., 0.03, 200.0)?;

                    ctx = execute_until_yields(&program, ctx, 1)?;

                    check_pumps(&mut ctx, true, true, true, 108.);
                }


                //ctx.debug_dump();
            }
        }

        Ok(())
    }

}
