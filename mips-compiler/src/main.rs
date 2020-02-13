use std::str::SplitWhitespace;
use std::collections::HashMap;
use crate::ParsedLine::OpCode;
use std::fmt::{Formatter, Error};

mod problem;

static PROG1:&str = include_str!("prog1.mips");

static PROG2:&str = include_str!("prog2.mips");

type InstructionPointer = u16;

type DeviceState = HashMap<String, f32>;

//

struct CPUContext
{
    labels: HashMap<String, InstructionPointer>,
    instruction_pointer:InstructionPointer,
    aliases: HashMap<String, RegisterOrDevice>,
    defines: HashMap<String, f32>,
    devices: Vec< Option<DeviceState> >,
    device_b: DeviceState,
    registers: Vec<f32>,
    saw_yield: bool,
}

impl CPUContext
{
    fn new(labels: HashMap<String,InstructionPointer>, aliases: HashMap<String,RegisterOrDevice>,
    devices:Vec<Option<DeviceState>>,
    registers:Vec<f32>) ->CPUContext
    {
        CPUContext {
            labels:labels,
            instruction_pointer: 0,
            aliases:aliases,
            defines: HashMap::new(),
            devices: devices,
            device_b: DeviceState::new(),
            registers: registers,
            saw_yield: false,
        }
    }

    fn lookup(&self, label:&LineNumber) -> Result<InstructionPointer, ExecutionError>
    {
        match label {
            LineNumber::Label(label) => {
                let tmp = self.labels.get(label);
                match tmp {
                    None => Err(ExecutionError { message: format!("no label '{}'", label) }),
                    Some(&number) => Ok(number)
                }
            },
            LineNumber::Number(number) => {
                Ok(*number)
            }
        }
    }

    fn resolve_device(&self, token: &AliasOrDevice) -> Result<Device, ExecutionError>
    {
        match token {
            AliasOrDevice::Alias(name) => {
                let thing = self.aliases.get(name);
                match thing {
                    None => {
                        Err(ExecutionError::new(&format!("unable to resolve alias {}", name)))
                    }
                    Some(thing) => {
                        match thing {
                            RegisterOrDevice::Register(reg) => {
                                Err(ExecutionError::new(&format!("{} is a register ({}) when I need a device", name, reg)))
                            },
                            RegisterOrDevice::Device(dev) => {
                                Ok(*dev)
                            }
                        }
                    }
                }

            },
            AliasOrDevice::Device(dev) => Ok(*dev),
        }
    }

    fn resolve_r_value(&self, r_value: &RValue) -> Result<f32, ExecutionError>
    {
        match r_value {
            RValue::Number(val) => Ok(*val),
            RValue::Register(reg) => {
                let ridx = reg.idx as usize;
                if ridx < self.registers.len() {
                    Ok(self.registers[ridx])
                } else {
                    Err(ExecutionError::new(&format!("no register {}", reg)))
                }
            },
            RValue::Name(tag) => {
                if let Some(rod) = self.aliases.get(tag) {
                    return match rod {
                        RegisterOrDevice::Device(dev) =>Err(ExecutionError::new(&format!("device alias {}={} can not be an rvalue", tag, dev))),
                        RegisterOrDevice::Register(reg) => self.register_reference(*reg)
                    };
                }
                match self.defines.get(tag) {
                    None => Err(ExecutionError::new(&format!("unable to evaluate {}", tag))),
                    Some(&val) => Ok(val),
                }
            },
        }
    }

    fn resolve_l_value(&self, l_value: &LValue) -> Result<Register, ExecutionError>
    {
        match l_value {
            LValue::Register(reg) => { Ok(*reg) },
            LValue::Alias(tag) => {
                match self.aliases.get(tag) {
                    None => Err(ExecutionError::new(&format!("not a valid LValue: {}", tag) )),
                    Some(rod) => {
                        match rod {
                            RegisterOrDevice::Register(reg) => Ok(*reg),
                            RegisterOrDevice::Device(dev) => Err(ExecutionError::new(&format!("{}={} is a device which is not a valid LValue", tag, dev)))
                        }
                    }
                }
            }
        }
    }

    fn attach_device(&mut self, idx:usize , dev:DeviceState) -> Result<(), ExecutionError>
    {
        if idx < self.devices.len() {
            self.devices[idx] = Some(dev);
            Ok(())
        } else {
            Err(ExecutionError::new(&format!("no device slot d{} on CPU", idx)))
        }
    }

    fn ip_plus_one(&mut self)
    {
        self.instruction_pointer+=1;
    }

    fn jump(&mut self, line_number:InstructionPointer)
    {
        self.instruction_pointer = line_number;
    }

    fn set_alias(&mut self, handle: &str, d_line: &RegisterOrDevice, incr_ip: bool)
    {
        self.aliases.insert(handle.to_string(), *d_line);
        if incr_ip {
            self.instruction_pointer += 1;
        }
    }

    fn set_define(&mut self, tag: &str, value: f32, incr_ip: bool)
    {
        self.defines.insert(tag.to_string(), value);
        if incr_ip {
            self.instruction_pointer += 1;
        }
    }

    fn device_reference(&mut self, dev:Device) -> Result<&mut DeviceState, ExecutionError>
    {
        match dev {
            Device::Regular(idx) => {
                if (idx as usize)<self.devices.len() {
                    match self.devices[idx as usize] {
                        Some(ref mut dev) => Ok(dev),
                        None => Err(ExecutionError::new(&format!("no device attached to d{}", idx)))

                    }
                } else {
                    Err(ExecutionError::new(&format!("no device d{}", idx)))
                }
            },
            Device::SpecialB => Ok(&mut self.device_b)
        }
    }

    fn get_device_field(&mut self, dev_idx:u8, field:&str) -> Result<f32, ExecutionError>
    {
        let dev = Device::Regular(dev_idx);
        let val = self.device_reference(dev)?.get(field);
        match val {
            Some(&val) => Ok(val),
            None => Err(ExecutionError::new(&format!("device {} has no field {}", dev, field))),
        }
    }

    fn set_device(&mut self, device:Device, field: &str, value: f32) -> Result<(), ExecutionError>
    {
        self.instruction_pointer+=1;
        self.device_reference(device)
            .map(|dev|  {
                dev.insert(field.to_string(), value);
            })
        /*
        match self.device_reference(device) {
            Ok(mut dev) => {
                dev.insert(field.to_string(), value);
                self.instruction_pointer+=1;
                Ok(())
            },
            Err(e)
        }
        let mut tmp = self.devices.get_mut(device.idx as usize);
        match tmp {
            Some(ref mut tmp) => {

                match tmp {
                    Some(ref mut dev ) => {
                        dev.insert(field.to_string(), value);
                        self.instruction_pointer+=1;
                        Ok(())
                    }
                    None => {
                        return Err(ExecutionError::new(&format!("no device attached to pin {}", device.idx)));
                    }
                }
            },
            None => {
                return Err(ExecutionError::new(&format!("bad device index {}", device.idx)))
            }
        }*/
    }

    fn register_reference(&self, reg:Register) -> Result<f32, ExecutionError>
    {
        if (reg.idx as usize) >= self.registers.len() {
            return Err(ExecutionError::new(&format!("no register {}", reg)))
        }
        Ok(self.registers[reg.idx as usize])
    }

    fn register_reference_mut(&mut self, reg:Register) -> Result<&mut f32, ExecutionError>
    {
        if (reg.idx as usize) >= self.registers.len() {
            return Err(ExecutionError::new(&format!("no register {}", reg)))
        }
        Ok(&mut self.registers[reg.idx as usize])
    }

    fn load_device(&mut self, reg:Register, dev: Device, tag: &str) -> Result<(), ExecutionError>
    {
        if (reg.idx as usize) >= self.registers.len() {
            return Err(ExecutionError::new(&format!("no register {}", reg)))
        }
        match self.device_reference(dev) {
            Ok(dev_state) => {
                let maybe_val = dev_state.get(tag);
                match maybe_val {
                    None => Err(ExecutionError::new(&format!("{}[{}] has no value", dev, tag))),
                    Some(&val) => {
                        self.registers[reg.idx as usize] = val;
                        self.ip_plus_one();
                        Ok(())
                    }
                }
            },
            Err(e) => Err(e),
        }
/*
        if (dev.idx as usize) < self.devices.len() {
            let devo = &self.devices[dev.idx as usize];
            match devo {
                None => Err(ExecutionError::new(&format!("device {} is not connected", dev))),
                Some(dev_state) => {

                }
            }
        } else {
            Err(ExecutionError::new(&format!("no such device d{}", dev.idx)))
        }
        */
    }

    fn yield_(&mut self)
    {
        self.saw_yield = true;
        self.ip_plus_one();
    }

    fn reset_yield(&mut self) ->bool
    {
        let rval = self.saw_yield;
        self.saw_yield = false;
        return rval;
    }

    fn debug_dump(&self)
    {
        println!("IP = {}", self.instruction_pointer);
        println!("labels = {:?}", self.labels);
        println!("aliases = {:?}", self.aliases);
        println!("devices = {:?} db={:?}", self.devices, self.device_b);
        println!("registers = {:?}", self.registers);
    }
}

//

#[derive(Debug,Clone)]
enum LineNumber
{
    Number(InstructionPointer),
    Label(String),
}

impl LineNumber
{
    fn parse(text: &str) -> Result<LineNumber,CompileError>
    {
        if let Ok(number) = text.parse::<InstructionPointer>() {
            Ok(LineNumber::Number(number))
        }  else {
            Ok(LineNumber::Label(text.to_string()))
        }
    }
}

//

struct CompileError
{
    message: String,
}

#[derive(Debug)]
struct ExecutionError
{
    message: String,
}

impl ExecutionError
{
    fn new(msg:&str) ->ExecutionError
    {
        ExecutionError{message:String::from(msg)}
    }
}

//

#[derive(Copy,Clone,Debug)]
struct Register
{
    idx:u8
}

impl std::fmt::Display for Register
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "r{}", self.idx)
    }
}

#[derive(Copy,Clone,Debug)]
enum Device
{
    Regular(u8),
    SpecialB
}

impl Device
{
    fn parse(tag:&str) -> Result<Device, CompileError>
    {
        if tag.starts_with("d") {
            if "db" == tag {
                return Ok(Device::SpecialB);
            }
            let idx = tag[1..].parse::<u8>();
            let idx = match idx {
                Ok(number) => number,
                Err(_) => {
                    return Result::Err(CompileError { message: format!("couldn't parse data line reference {}", tag) });
                }
            };
            Ok(Device::Regular(idx))
        } else {
            Err(CompileError{message: format!("not a device")})
        }
    }
}

impl std::fmt::Display for Device
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            Device::Regular(idx) => write!(f, "d{}", idx),
            Device::SpecialB => write!(f, "db"),
        }
    }
}

#[derive(Copy,Clone,Debug)]
enum RegisterOrDevice
{
    Register(Register),
    Device(Device)
}

impl RegisterOrDevice
{
    fn parse(tag: &str) -> Result<RegisterOrDevice, CompileError>
    {
        if let Ok(dev) = Device::parse(tag) {
            Ok(RegisterOrDevice::Device(dev))
        } else if tag.starts_with("r") {
            let idx = tag[1..].parse::<u8>();
            match idx {
                Ok(number) =>
                    Ok( RegisterOrDevice::Register(Register{idx:number}) ),
                Err(_) => {
                    return Result::Err(CompileError{message: format!("couldn't parse data line reference {}", tag)});
                }
            }
        } else {
            Err(CompileError{message:"was expecting a register or data line reference".to_string()})
        }
    }
}

impl std::fmt::Display for RegisterOrDevice
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            RegisterOrDevice::Register(rx) =>
                write!(f, "{}", rx),
            RegisterOrDevice::Device(rx) =>
                write!(f, "{}", rx),
        }
    }
}

//

enum AliasOrDevice
{
    Alias(String),
    Device(Device),
}

impl AliasOrDevice
{
    fn parse(text:&str) -> Result<AliasOrDevice, CompileError>
    {
        if let Ok(dev) = Device::parse(text) {
            Ok(AliasOrDevice::Device(dev))
        } else {
            Ok(AliasOrDevice::Alias(text.to_string()))
        }
    }
}

//

enum RValue
{
    Number(f32),
    Register(Register),
    Name(String),
}

impl RValue
{
    fn parse(text: &str) -> Result<RValue, CompileError>
    {
        if let Ok(val) = text.parse::<f32>() {
            Ok(RValue::Number(val))
        } else if text.starts_with("r") {
            if let Ok(idx) = text[1..].parse::<u8>() {
                Ok(RValue::Register(Register{idx:idx}))
            } else {
                Err(CompileError{ message: format!("unable to parse '{}' to a number or register", text)})
            }
        } else {
            Ok(RValue::Name(text.to_string()))
            //Err(CompileError{ message: format!("unable to parse '{}' to a number or register", text)})
        }
    }
}

//

enum LValue
{
    Register(Register),
    Alias(String),
}

impl LValue
{
    fn parse(text:&str) -> Result<LValue, CompileError>
    {
        if text.starts_with("r") {
            if let Ok(val) = text[1..].parse::<u8>() {
                return Ok(LValue::Register(Register{idx:val}));
            }
        }
        Ok(LValue::Alias(text.to_string()))
    }
}

//

trait Instruction
{
    fn execute(&self, ctx: CPUContext) -> Result<CPUContext, ExecutionError>;
}

//

struct NoCode {}

impl Instruction for NoCode
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError>
    {
        ctx.ip_plus_one();
        Ok(ctx)
    }
}

struct UnrecognizedOpcode
{
    opcode:String,
}

impl Instruction for UnrecognizedOpcode
{
    fn execute(&self, _ctx: CPUContext) -> Result<CPUContext, ExecutionError>
    {
        Err( ExecutionError::new(&format!("unrecognized opcode {}", self.opcode)) )
    }
}

//

struct Jump
{
    line_number: LineNumber
}

impl Jump
{
    fn new(mut parts: SplitWhitespace) ->Result<Jump, CompileError>
    {
        let generic_error = "'j' jump instruction requires 1 argument of line number or label".to_string();
        let tgt= parts.next();
        //println!("tgt = {:?}", tgt);
        match tgt {
            None => Err(CompileError { message: generic_error}),
            Some(val) => {

                let expect_none = parts.next();
                //println!("none = {:?}", expect_none);
                match expect_none {
                    Some(_) => return Err(CompileError { message: generic_error}),
                    None => {}
                }

                let a = val.parse::<InstructionPointer>();

                let line_number = match a {
                    Ok(number) => {
                        LineNumber::Number(number)
                    },
                    Err(_) => {
                        LineNumber::Label(String::from(val))
                    }
                };

                Ok( Jump{ line_number: line_number} )

            }
        }

    }
}

impl Instruction for Jump
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError> {
        let line_number = ctx.lookup(&self.line_number)?;
        ctx.jump(line_number);
        Ok(ctx)
    }
}

//

fn expect_2<'a, I>(mut parts: I) -> Result<(String, String), CompileError>
    where I:Iterator<Item=&'a str>
{
    let one = parts.next();
    let two = parts.next();
    let doom = parts.next();

    if let (Some(one), Some(two), None) = (one,two, doom) {
        Ok((one.to_string(),two.to_string()))
    } else {
        Err(CompileError{ message: "expected 2 arguments".to_string()})
    }
}

fn expect_3<'a, I>(mut parts: I) -> Result<(String, String, String), CompileError>
    where I:Iterator<Item=&'a str>
{
    let one = parts.next();
    let two = parts.next();
    let three = parts.next();
    let doom = parts.next();

    if let (Some(one), Some(two), Some(three), None) = (one,two, three, doom) {
        Ok((one.to_string(),two.to_string(), three.to_string()))
    } else {
        Err(CompileError{ message: "expected 3 arguments".to_string()})
    }
}

fn expect_4<'a, I>(mut parts: I) -> Result<(String, String, String, String), CompileError>
    where I:Iterator<Item=&'a str>
{
    let one = parts.next();
    let two = parts.next();
    let three = parts.next();
    let four = parts.next();
    let doom = parts.next();

    if let (Some(one), Some(two), Some(three), Some(four), None) = (one,two,three,four, doom) {
        Ok((one.to_string(),two.to_string(), three.to_string(), four.to_string()))
    } else {
        Err(CompileError{ message: "expected 4 arguments".to_string()})
    }
}

//

struct Alias
{
    handle: String,
    d_line: RegisterOrDevice,
}

impl Alias
{
    fn new(parts: SplitWhitespace) -> Result<Alias, CompileError>
    {
        let (label, d_line) = expect_2(parts)?;
        let d_line = RegisterOrDevice::parse(&d_line)?;
        Ok(Alias { handle:label, d_line:d_line})
    }
}

impl Instruction for Alias
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError> {
        ctx.set_alias(&self.handle, &self.d_line, true);
        Ok(ctx)
    }
}

//

struct Define
{
    tag:String,
    value:f32,
}

impl Define
{

    fn new(parts: SplitWhitespace) -> Result<Define, CompileError>
    {
        let (tag, value) = expect_2(parts)?;
        match value.parse::<f32>() {
            Ok(value) =>
                Ok(Define { tag: tag, value: value }),
            Err(_) => Err(CompileError{message:format!("failed to parse value '{}' in define", value)}),
        }
    }
}

impl Instruction for Define
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError> {
        ctx.set_define(&self.tag, self.value, true);
        Ok(ctx)
    }
}

//

struct SetDevice
{
    device: AliasOrDevice,
    field: String,
    r_value: RValue,
}

impl SetDevice
{
    fn new<'a, I>(parts: I ) -> Result<SetDevice, CompileError>
        where I:Iterator<Item=&'a str>
    {
        let (dev, tag, r_value) = expect_3(parts)?;

        Ok(SetDevice{
            device: AliasOrDevice::parse(&dev)?,
            field: tag,
            r_value: RValue::parse(&r_value)?,
        })
    }
}

impl Instruction for SetDevice
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError> {
        ctx.set_device(ctx.resolve_device(&self.device)?,
                       &self.field,
                       ctx.resolve_r_value(&self.r_value)?)?;
        Ok(ctx)
    }
}

//

struct LoadDevice
{
    l_value: LValue,
    device: AliasOrDevice,
    field: String,
}

impl LoadDevice
{
    fn new<'a, I>(parts: I ) -> Result<LoadDevice, CompileError>
        where I:Iterator<Item=&'a str>
    {
        let (l_value, dev, tag) = expect_3(parts)?;

        Ok(LoadDevice{
            l_value: LValue::parse(&l_value)?,
            device: AliasOrDevice::parse(&dev)?,
            field: tag,
        })
    }
}

impl Instruction for LoadDevice
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError>
    {
        ctx.load_device(ctx.resolve_l_value(&self.l_value)?,
                        ctx.resolve_device(&self.device)?,
                        &self.field)?;
        Ok(ctx)
    }
}

//

struct Move
{
    l_value: LValue,
    r_value: RValue,
}

impl Move
{

    fn new<'a, I>(mut parts: I ) -> Result<Move, CompileError>
        where I:Iterator<Item=&'a str>
    {
        let (l_value, r_value) = expect_2(parts)?;

        Ok(Move{
            l_value: LValue::parse(&l_value)?,
            r_value: RValue::parse(&r_value)?,
        })
    }
}

impl Instruction for Move
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError> {
        let src = ctx.resolve_r_value(&self.r_value)?;
        let dst = ctx.resolve_l_value(&self.l_value)?;
        *ctx.register_reference_mut(dst)? = src;
        Ok(ctx)
    }
}

//
/*
struct Subtract
{
    l_value: LValue,
    arg1: RValue,
    arg2: RValue,
}

impl Subtract
{
    fn new<'a, I>(parts: I ) -> Result<Subtract, CompileError>
        where I:Iterator<Item=&'a str>
    {
        let (l_value, arg1, arg2) = expect_3(parts)?;

        Ok(Subtract{
            l_value: LValue::parse(&l_value)?,
            arg1: RValue::parse(&arg1)?,
            arg2: RValue::parse(&arg2)?,
        })
    }
}

impl Instruction for Subtract
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError>
    {
        let val = ctx.resolve_r_value(&self.arg1)? - ctx.resolve_r_value(&self.arg2)?;
        let dst = ctx.register_reference(ctx.resolve_l_value(&self.l_value)?)?;
        *dst = val;
        Ok(ctx)


    }
}
*/

//

struct BinaryOperator
{
    l_value: LValue,
    arg1: RValue,
    arg2: RValue,
    op: Box<dyn Fn(f32,f32)->f32>,
}

impl BinaryOperator
{
    fn new<'a, I, F>(parts: I , op:F) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>,
              F: Fn(f32,f32)->f32 +'static
    {
        let (l_value, arg1, arg2) = expect_3(parts)?;

        Ok(BinaryOperator{
            l_value: LValue::parse(&l_value)?,
            arg1: RValue::parse(&arg1)?,
            arg2: RValue::parse(&arg2)?,
            op: Box::new(op),
        })
    }

    fn add<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| a+b)
    }

    fn sub<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| a-b)
    }

    fn div<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| a/b)
    }

    fn modulus<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| a%b)
    }

    fn max<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| a.max(b))
    }

    fn min<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| a.min(b))
    }

    fn slt<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| if a<b { 1.0 } else {0.0} )
    }

    fn sgt<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| if a>b { 1.0 } else {0.0} )
    }

}

impl Instruction for BinaryOperator
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError>
    {
        let a = ctx.resolve_r_value(&self.arg1)?;
        let b = ctx.resolve_r_value(&self.arg2)?;
        let dst = ctx.register_reference_mut(ctx.resolve_l_value(&self.l_value)?)?;
        *dst = (self.op)(a, b);
        ctx.ip_plus_one();
        Ok(ctx)
    }
}

//

struct TernaryOperator
{
    l_value: LValue,
    arg1: RValue,
    arg2: RValue,
    arg3: RValue,
    op: Box<dyn Fn(f32,f32,f32)->f32>,
}

impl TernaryOperator
{
    fn new<'a, I, F>(parts: I , op:F) -> Result<TernaryOperator, CompileError>
        where I:Iterator<Item=&'a str>,
              F: Fn(f32,f32,f32)->f32 +'static
    {
        let (l_value, arg1, arg2, arg3) = expect_4(parts)?;

        Ok(TernaryOperator{
            l_value: LValue::parse(&l_value)?,
            arg1: RValue::parse(&arg1)?,
            arg2: RValue::parse(&arg2)?,
            arg3: RValue::parse(&arg3)?,
            op: Box::new(op),
        })
    }

    fn select<'a, I>(parts:I) -> Result<TernaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        TernaryOperator::new(parts, |a,b, c| if a!=0.0 {b} else {c} )
    }

}

impl Instruction for TernaryOperator
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError>
    {
        let a = ctx.resolve_r_value(&self.arg1)?;
        let b = ctx.resolve_r_value(&self.arg2)?;
        let c = ctx.resolve_r_value(&self.arg3)?;
        let dst = ctx.register_reference_mut(ctx.resolve_l_value(&self.l_value)?)?;
        *dst = (self.op)(a, b, c);
        ctx.ip_plus_one();
        Ok(ctx)
    }
}

//

struct Branch
{
    arg1: RValue,
    arg2: RValue,
    target: LineNumber,
    op: Box<dyn Fn(f32,f32)->bool>,
}

impl Branch
{
    fn new<'a, I, F>(parts: I , op:F) -> Result<Branch, CompileError>
        where I:Iterator<Item=&'a str>,
              F: Fn(f32,f32)->bool +'static
    {
        let (arg1, arg2, target) = expect_3(parts)?;

        Ok(Branch{
            arg1: RValue::parse(&arg1)?,
            arg2: RValue::parse(&arg2)?,
            target: LineNumber::parse(&target)?,
            op: Box::new(op),
        })
    }

    fn gt<'a, I>(parts:I) -> Result<Branch, CompileError>
        where I:Iterator<Item=&'a str>
    {
        Branch::new(parts, |a,b| a>b)
    }

}

impl Instruction for Branch
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError>
    {
        let a = ctx.resolve_r_value(&self.arg1)?;
        let b = ctx.resolve_r_value(&self.arg2)?;

        let result = (self.op)(a,b);
        if result {
            ctx.instruction_pointer = ctx.lookup(&self.target)?;
        } else {
            ctx.instruction_pointer += 1;
        }
        Ok(ctx)
    }
}

//

struct BranchDevice
{
    dev: AliasOrDevice,
    target: LineNumber,
    predicate: Box<dyn Fn(&CPUContext, Device)->bool>,
}

impl BranchDevice
{
    fn new<'a, I, F>(parts: I , op:F) -> Result<BranchDevice, CompileError>
        where I:Iterator<Item=&'a str>,
//              F: Fn(f32,f32)->bool +'static
              F: Fn(&CPUContext, Device)->bool +'static
    {
        let (arg1, target) = expect_2(parts)?;

        Ok(BranchDevice{
            dev: AliasOrDevice::parse(&arg1)?,
            target: LineNumber::parse(&target)?,
            predicate: Box::new(op),
        })
    }

    fn device_not_set(ctx : &CPUContext, dev: Device) ->bool
    {
        match dev {
            Device::SpecialB => true,
            Device::Regular(idx) => {
                match ctx.devices.get(idx as usize) {
                    Some(_) => false,
                    None => true,
                }
            }
        }

    }

    fn bdns<'a,I>(parts:I) ->Result<BranchDevice, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BranchDevice::new(parts, BranchDevice::device_not_set)
    }
}

impl Instruction for BranchDevice
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError> {
        let dev = ctx.resolve_device(&self.dev)?;
        if (self.predicate)(&ctx, dev) {
            ctx.instruction_pointer = ctx.lookup(&self.target)?;
        } else {
            ctx.ip_plus_one();
        }
        Ok(ctx)
    }
}

//

struct Yield { }

impl Instruction for Yield
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError> {
        ctx.yield_();
        Ok(ctx)
    }
}

//

impl<T:Instruction+'static> From<Result<T, CompileError>> for ParsedLine
{
    fn from(x: Result<T, CompileError>) -> Self
    {
        match x {
            Ok(a) => OpCode(Box::new(a)),
            Err(e) => ParsedLine::Err(e),
        }
    }
}


//

enum ParsedLine
{
    OpCode(Box<dyn Instruction>),
    JumpLabel(String),
    Err(CompileError)
}

//

fn parse_one_line(line:&str) -> ParsedLine
{
    let line = {
        if let Some(idx) = line.find('#') {
            &line[..idx]
        } else {
            line
        }
    };
    let idx = line.find(':');
    if let Some(idx) = idx {
        return ParsedLine::JumpLabel(line[..idx].trim().to_string());
    }

    let mut parts = line.trim().split_whitespace();

    let opcode = parts.next();
    return match opcode {
        None => {
            let x:Box<dyn Instruction> = Box::new(NoCode{});
            ParsedLine::OpCode(x)
        },

        Some(opcode)=> {
            if "j" == opcode {
                Jump::new(parts).into()
            } else if "alias" == opcode {
                Alias::new(parts).into()
            } else if "define" == opcode {
                Define::new(parts).into()
            } else if "s" == opcode {
                SetDevice::new(parts).into()
            } else if "yield" == opcode {
                ParsedLine::OpCode(Box::new(Yield {}))
            } else if "l" == opcode {
                LoadDevice::new(parts).into()
            } else if "move" == opcode {
                Move::new(parts).into()

            } else if "sub" == opcode {
                BinaryOperator::sub(parts).into()
            } else if "add" == opcode {
                BinaryOperator::add(parts).into()
            } else if "div" == opcode {
                BinaryOperator::div(parts).into()
            } else if "mod" == opcode {
                BinaryOperator::modulus(parts).into()
            } else if "max" == opcode {
                BinaryOperator::max(parts).into()
            } else if "min" == opcode {
                BinaryOperator::min(parts).into()

            } else if "slt" == opcode {
                BinaryOperator::slt(parts).into()
            } else if "sgt" == opcode {
                BinaryOperator::sgt(parts).into()

            } else if "select" == opcode {
                TernaryOperator::select(parts).into()

            } else if "bgt" == opcode {
                Branch::gt(parts).into()

            } else if "bdns" == opcode {
                BranchDevice::bdns(parts).into()

            } else {
                ParsedLine::Err(CompileError{message: format!("unrecognized opcode {}", opcode)})
            }
        }

    };
}

//

struct DeviceStateBuilder
{
    rval : DeviceState,
}

impl DeviceStateBuilder
{
    fn new() -> DeviceStateBuilder
    {
        DeviceStateBuilder{rval:DeviceState::new()}
    }

    fn set(mut self, field:&str, value:f32) -> DeviceStateBuilder
    {
        self.rval .insert(field.to_string(), value);

        self
    }

    fn build(self) -> DeviceState
    {
        self.rval
    }
}

//

struct CompiledProgram
{
    codes: Vec<Box<dyn Instruction>>,
    labels: HashMap<String, InstructionPointer>,
}

impl CompiledProgram
{
    fn labels(&self) -> HashMap<String,InstructionPointer>
    {
        self.labels.clone()
    }

    fn get_instruction(&self, idx:InstructionPointer) -> Option<&dyn Instruction>
    {
        if (idx as usize) < self.codes.len() {
            Some(& * self.codes[idx as usize])
        } else {
            None
        }
    }
}


fn compile(src:&str) ->Result<CompiledProgram, CompileError>
{
    let lines:std::str::Lines = src.lines();

    compile_lines(lines)
}

fn compile_lines<'a,I>(lines: I) -> Result<CompiledProgram, CompileError>
    where I:Iterator<Item=&'a str>
{
    let mut codes2: Vec<Box<dyn Instruction>> = Vec::new();
    let mut labels: HashMap<String, InstructionPointer> = HashMap::new();
    let mut line_number = 0;
    for line in lines {
        let x = parse_one_line(line);
        let transformed = match x {
            ParsedLine::OpCode(op_code) => op_code,
            ParsedLine::JumpLabel(jump_label) => {
                labels.insert(jump_label, line_number);
                Box::new(NoCode {})
            },
            ParsedLine::Err(e) => {
                println!("{}: {}", line_number, line);
                return Err(e)
            }
        };
        codes2.push(transformed);
        line_number += 1;
    }
    return Ok(CompiledProgram {
        codes: codes2,
        labels: labels,
    });
}

//
//
//

fn execute_until_yields(program:&CompiledProgram, mut ctx:CPUContext, max_yields:u32) -> Result<CPUContext, ExecutionError>
{
    let mut yield_count=0;
    for _i in 0..99 {
        let inst = program.get_instruction(ctx.instruction_pointer);
        if let None = inst {
            println!("reached end of program");
            break;
        }
        ctx = inst.unwrap().execute(ctx)?;
        if ctx.reset_yield() {
            yield_count+=1;
            if yield_count >= max_yields {
                break;
            }
        }
        println!("IP = {}", ctx.instruction_pointer)
    }
    Ok(ctx)
}

fn execute_until_yields2<F>(program:&CompiledProgram, mut ctx:CPUContext, max_yields:u32, callback:F) -> Result<CPUContext, ExecutionError>
    where F:Fn(&mut CPUContext)
{
    let mut yield_count=0;
    for _i in 0..99 {
        let inst = program.get_instruction(ctx.instruction_pointer);
        if let None = inst {
            println!("reached end of program");
            break;
        }
        ctx = inst.unwrap().execute(ctx)?;
        callback(&mut ctx);
        if ctx.reset_yield() {
            yield_count+=1;
            if yield_count >= max_yields {
                break;
            }
        }
        println!("IP = {}", ctx.instruction_pointer)
    }
    Ok(ctx)
}

/*
fn main() -> Result<(), ExecutionError> {
    println!("Hello, world!");
    if false {
        print!("{}", PROG1);
    }

    test_prog2()?;
    if false {
        test_prog1()?;
    }

    Ok(())
}
*/

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
                                                                        if ctx.instruction_pointer >16 && ctx.instruction_pointer < 21 {
                                                                            ctx.debug_dump();
                                                                        }
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