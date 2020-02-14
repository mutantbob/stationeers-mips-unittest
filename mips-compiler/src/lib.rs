use std::collections::HashMap;
use std::fmt::{Formatter, Error};

pub type InstructionPointer = u16;

pub type DeviceState = HashMap<String, f32>;

#[cfg(test)]
mod tests;

//

pub struct CPUContext
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
    pub fn new_simple(program:&CompiledProgram) -> CPUContext
    {
        CPUContext::new(program.labels(), HashMap::new(),
                        (0..6).map(|_| None).collect(),
                        (0..18).map(|_| std::f32::NAN).collect())
    }

    pub fn new(labels: HashMap<String,InstructionPointer>, aliases: HashMap<String,RegisterOrDevice>,
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

    pub fn lookup(&self, label:&LineNumber) -> Result<InstructionPointer, ExecutionError>
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

    pub fn resolve_device(&self, token: &AliasOrDevice) -> Result<Device, ExecutionError>
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

    pub fn resolve_r_value(&self, r_value: &RValue) -> Result<f32, ExecutionError>
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

    pub fn resolve_l_value(&self, l_value: &LValue) -> Result<Register, ExecutionError>
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

    pub fn attach_device(&mut self, idx:usize , dev:DeviceState) -> Result<(), ExecutionError>
    {
        if idx < self.devices.len() {
            self.devices[idx] = Some(dev);
            Ok(())
        } else {
            Err(ExecutionError::new(&format!("no device slot d{} on CPU", idx)))
        }
    }

    pub fn ip_plus_one(&mut self)
    {
        self.instruction_pointer+=1;
    }

    pub fn get_ra(&self) -> f32
    {
        self.registers[self.registers.len()-1]
    }

    pub fn set_ra(&mut self, ptr:InstructionPointer)
    {
        let idx = self.registers.len() - 1;
        self.registers[idx] = ptr as f32;
    }

    pub fn jump(&mut self, line_number:InstructionPointer)
    {
        self.instruction_pointer = line_number;
    }

    pub fn set_alias(&mut self, handle: &str, d_line: &RegisterOrDevice, incr_ip: bool)
    {
        self.aliases.insert(handle.to_string(), *d_line);
        if incr_ip {
            self.instruction_pointer += 1;
        }
    }

    pub fn set_define(&mut self, tag: &str, value: f32, incr_ip: bool)
    {
        self.defines.insert(tag.to_string(), value);
        if incr_ip {
            self.instruction_pointer += 1;
        }
    }

    pub fn device_reference(&mut self, dev:Device) -> Result<&mut DeviceState, ExecutionError>
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

    pub fn get_device_field(&mut self, dev_idx:u8, field:&str) -> Result<f32, ExecutionError>
    {
        let dev = Device::Regular(dev_idx);
        let val = self.device_reference(dev)?.get(field);
        match val {
            Some(&val) => Ok(val),
            None => Err(ExecutionError::new(&format!("device {} has no field {}", dev, field))),
        }
    }

    pub fn set_device(&mut self, device:Device, field: &str, value: f32) -> Result<(), ExecutionError>
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

    pub fn register_reference(&self, reg:Register) -> Result<f32, ExecutionError>
    {
        if (reg.idx as usize) >= self.registers.len() {
            return Err(ExecutionError::new(&format!("no register {}", reg)))
        }
        Ok(self.registers[reg.idx as usize])
    }

    pub fn register_reference_mut(&mut self, reg:Register) -> Result<&mut f32, ExecutionError>
    {
        if (reg.idx as usize) >= self.registers.len() {
            return Err(ExecutionError::new(&format!("no register {}", reg)))
        }
        Ok(&mut self.registers[reg.idx as usize])
    }

    pub fn load_device(&mut self, reg:Register, dev: Device, tag: &str) -> Result<(), ExecutionError>
    {
        if (reg.idx as usize) >= self.registers.len() {
            return Err(ExecutionError::new(&format!("no register {}", reg)))
        }
        match self.device_reference(dev) {
            Ok(dev_state) => {
                let maybe_val = dev_state.get(tag);
                if false {
                    // the javascript simulator doesn't work like this.  It just loads 0
                    match maybe_val {
                        None => Err(ExecutionError::new(&format!("{}[{}] has no value", dev, tag))),
                        Some(&val) => {
                            self.registers[reg.idx as usize] = val;
                            self.ip_plus_one();
                            Ok(())
                        }
                    }
                } else {
                    let val = maybe_val.unwrap_or(&0_f32);
                    self.registers[reg.idx as usize] = *val;
                    self.ip_plus_one();
                    Ok(())
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

    pub fn yield_(&mut self)
    {
        self.saw_yield = true;
        self.ip_plus_one();
    }

    pub fn reset_yield(&mut self) ->bool
    {
        let rval = self.saw_yield;
        self.saw_yield = false;
        return rval;
    }

    pub fn debug_dump(&self)
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
pub enum LineNumber
{
    Number(InstructionPointer),  // this can't be negative.  Should we allow that for relative branching?
    Label(String),
}

impl LineNumber
{
    pub fn parse(text: &str) -> Result<LineNumber,CompileError>
    {
        if let Ok(number) = text.parse::<InstructionPointer>() {
            Ok(LineNumber::Number(number))
        }  else {
            Ok(LineNumber::Label(text.to_string()))
        }
    }
}

//

#[derive(Debug)]
pub struct CompileError
{
    pub message: String,
}

#[derive(Debug)]
pub struct ExecutionError
{
    message: String,
}

impl ExecutionError
{
    pub fn new(msg:&str) ->ExecutionError
    {
        ExecutionError{message:String::from(msg)}
    }
}

//

#[derive(Debug)]
pub enum MultiError
{
    Compile(CompileError),
    Execution(ExecutionError),
}

impl From<CompileError> for MultiError
{
    fn from(e: CompileError) -> Self {
        MultiError::Compile(e)
    }
}

impl From<ExecutionError> for MultiError
{
    fn from(e: ExecutionError) -> Self {
        MultiError::Execution(e)
    }
}

//

#[derive(Copy,Clone,Debug)]
pub struct Register
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
pub enum Device
{
    Regular(u8),
    SpecialB
}

impl Device
{
    pub fn parse(tag:&str) -> Result<Device, CompileError>
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
pub enum RegisterOrDevice
{
    Register(Register),
    Device(Device)
}

impl RegisterOrDevice
{
    pub fn parse(tag: &str) -> Result<RegisterOrDevice, CompileError>
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

pub enum AliasOrDevice
{
    Alias(String),
    Device(Device),
}

impl AliasOrDevice
{
    pub fn parse(text:&str) -> Result<AliasOrDevice, CompileError>
    {
        if let Ok(dev) = Device::parse(text) {
            Ok(AliasOrDevice::Device(dev))
        } else {
            Ok(AliasOrDevice::Alias(text.to_string()))
        }
    }
}

//

pub enum RValue
{
    Number(f32),
    Register(Register),
    Name(String),
}

impl RValue
{
    pub fn parse(text: &str) -> Result<RValue, CompileError>
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

pub enum LValue
{
    Register(Register),
    Alias(String),
}

impl LValue
{
    pub fn parse(text:&str) -> Result<LValue, CompileError>
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

pub trait Instruction
{
    fn execute(&self, ctx: CPUContext) -> Result<CPUContext, ExecutionError>;
}

//

pub struct NoCode {}

impl Instruction for NoCode
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError>
    {
        ctx.ip_plus_one();
        Ok(ctx)
    }
}

pub struct UnrecognizedOpcode
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

pub struct Jump
{
    line_number: LineNumber
}

impl Jump
{
    pub fn new<'a,I>(mut parts: I) ->Result<Jump, CompileError>
        where I:Iterator<Item=&'a str>
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

pub fn expect_2<'a, I>(mut parts: I) -> Result<(String, String), CompileError>
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

pub fn expect_3<'a, I>(mut parts: I) -> Result<(String, String, String), CompileError>
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

pub fn expect_4<'a, I>(mut parts: I) -> Result<(String, String, String, String), CompileError>
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

pub struct Alias
{
    handle: String,
    d_line: RegisterOrDevice,
}

impl Alias
{
    pub fn new<'a,I>(parts: I) -> Result<Alias, CompileError>
        where I:Iterator<Item=&'a str>
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

pub struct Define
{
    tag:String,
    value:f32,
}

impl Define
{

    pub fn new<'a,I>(parts: I) -> Result<Define, CompileError>
        where I:Iterator<Item=&'a str>
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

pub struct SetDevice
{
    device: AliasOrDevice,
    field: String,
    r_value: RValue,
}

impl SetDevice
{
    pub fn new<'a, I>(parts: I ) -> Result<SetDevice, CompileError>
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

pub struct LoadDevice
{
    l_value: LValue,
    device: AliasOrDevice,
    field: String,
}

impl LoadDevice
{
    pub fn new<'a, I>(parts: I ) -> Result<LoadDevice, CompileError>
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

pub struct Move
{
    l_value: LValue,
    r_value: RValue,
}

impl Move
{

    pub fn new<'a, I>(parts: I ) -> Result<Move, CompileError>
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
        ctx.ip_plus_one();
        Ok(ctx)
    }
}

//

pub struct BinaryOperator
{
    l_value: LValue,
    arg1: RValue,
    arg2: RValue,
    op: Box<dyn Fn(f32,f32)->f32>,
}

impl BinaryOperator
{
    pub fn new<'a, I, F>(parts: I , op:F) -> Result<BinaryOperator, CompileError>
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

    pub fn add<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| a+b)
    }

    pub fn sub<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| a-b)
    }

    pub fn div<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| a/b)
    }

    pub fn modulus<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| a%b)
    }

    pub fn max<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| a.max(b))
    }

    pub fn min<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| a.min(b))
    }

    pub fn slt<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BinaryOperator::new(parts, |a,b| if a<b { 1.0 } else {0.0} )
    }

    pub fn sgt<'a, I>(parts:I) -> Result<BinaryOperator, CompileError>
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

pub struct TernaryOperator
{
    l_value: LValue,
    arg1: RValue,
    arg2: RValue,
    arg3: RValue,
    op: Box<dyn Fn(f32,f32,f32)->f32>,
}

impl TernaryOperator
{
    pub fn new<'a, I, F>(parts: I , op:F) -> Result<TernaryOperator, CompileError>
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

    pub fn select<'a, I>(parts:I) -> Result<TernaryOperator, CompileError>
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

pub struct Branch
{
    arg1: RValue,
    arg2: RValue,
    target: LineNumber,
    op: Box<dyn Fn(f32,f32)->bool>,
}

impl Branch
{
    pub fn new<'a, I, F>(parts: I , op:F) -> Result<Branch, CompileError>
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

    pub fn gt<'a, I>(parts:I) -> Result<Branch, CompileError>
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

pub struct BranchDevice
{
    dev: AliasOrDevice,
    target: LineNumber,
    predicate: Box<dyn Fn(&CPUContext, Device)->Result<bool,ExecutionError>>,
    and_link: bool,
    relative: bool,
}

impl BranchDevice
{
    pub fn new<'a, I, F>(parts: I , op:F, and_link: bool, relative:bool) -> Result<BranchDevice, CompileError>
        where I:Iterator<Item=&'a str>,
//              F: Fn(f32,f32)->bool +'static
              F: Fn(&CPUContext, Device)->Result<bool,ExecutionError> +'static
    {
        let (arg1, target) = expect_2(parts)?;

        Ok(BranchDevice{
            dev: AliasOrDevice::parse(&arg1)?,
            target: LineNumber::parse(&target)?,
            predicate: Box::new(op),
            and_link: and_link,
            relative: relative,
        })
    }

    pub fn device_not_set(ctx : &CPUContext, dev: Device) ->Result<bool,ExecutionError>
    {
        return Ok(!BranchDevice::device_attached(ctx, dev)?);
    }

    pub fn device_attached(ctx : &CPUContext, dev: Device) ->Result<bool,ExecutionError>
    {
        //println!("bdns ? {}", dev);
        match dev {
            Device::SpecialB => Ok(true),
            Device::Regular(idx) => {
                let urgh = ctx.devices.get(idx as usize);
                match urgh {
                    Some(l2) => match l2 {
                        Some(_) => Ok(true),
                        None => Ok(false),
                    },
                    None => Err(ExecutionError::new(&format!("no such device slot d{}", idx))),
                }
            }
        }

    }

    pub fn bdns<'a,I>(parts:I) ->Result<BranchDevice, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BranchDevice::new(parts, BranchDevice::device_not_set, false, false)
    }

    pub fn bdnsal<'a,I>(parts:I) ->Result<BranchDevice, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BranchDevice::new(parts, BranchDevice::device_not_set, true, false)
    }

    pub fn bdse<'a,I>(parts:I) ->Result<BranchDevice, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BranchDevice::new(parts, BranchDevice::device_attached, false, false)
    }

    pub fn bdseal<'a,I>(parts:I) ->Result<BranchDevice, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BranchDevice::new(parts, BranchDevice::device_attached, true, false)
    }

    pub fn brdns<'a,I>(parts:I) ->Result<BranchDevice, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BranchDevice::new(parts, BranchDevice::device_not_set, false, true)
    }

    pub fn brdse<'a,I>(parts:I) ->Result<BranchDevice, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BranchDevice::new(parts, BranchDevice::device_attached, false, true)
    }
}

impl Instruction for BranchDevice
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError> {
        let dev = ctx.resolve_device(&self.dev)?;
        ctx.ip_plus_one();
        if (self.predicate)(&ctx, dev)? {
            if self.and_link {
                ctx.set_ra(ctx.instruction_pointer);
            }
            let target = ctx.lookup(&self.target)?;
            if self.relative {
                ctx.instruction_pointer = ctx.instruction_pointer - 1 + target;
            } else {
                ctx.instruction_pointer = target;
            }
        }
        Ok(ctx)
    }
}

//

pub struct BranchTernary
{
    arg1: RValue,
    arg2: RValue,
    frac: RValue,
    target: LineNumber,
    op: Box<dyn Fn(f32,f32,f32)->bool>,
    and_link: bool,
}

impl BranchTernary
{
    pub fn new<'a, I, F>(parts: I , op:F, and_link:bool) -> Result<BranchTernary, CompileError>
        where I:Iterator<Item=&'a str>,
              F: Fn(f32,f32,f32)->bool +'static
    {
        let (arg1, arg2, arg3, target) = expect_4(parts)?;

        Ok(BranchTernary{
            arg1: RValue::parse(&arg1)?,
            arg2: RValue::parse(&arg2)?,
            frac: RValue::parse(&arg3)?,
            target: LineNumber::parse(&target)?,
            op: Box::new(op),
            and_link:and_link,
        })
    }

    pub fn approximately_the_same(a:f32, b:f32, frac:f32) ->bool
    {
        // yeah, this is mildly confusing
        let margin1 = std::f32::EPSILON * 8.;
        let scale = a.abs().max(b.abs());
        let margin2 = frac * scale;
        let tolerance = margin1.max(margin2);
        (a-b).abs() < tolerance
    }

    pub fn bap<'a, I>(parts:I) -> Result<BranchTernary, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BranchTernary::new(parts, BranchTernary::approximately_the_same, false)
    }

    pub fn bapal<'a, I>(parts:I) -> Result<BranchTernary, CompileError>
        where I:Iterator<Item=&'a str>
    {
        BranchTernary::new(parts, BranchTernary::approximately_the_same, true)
    }

}

impl Instruction for BranchTernary
{
    fn execute(&self, mut ctx: CPUContext) -> Result<CPUContext, ExecutionError>
    {
        let a = ctx.resolve_r_value(&self.arg1)?;
        let b = ctx.resolve_r_value(&self.arg2)?;
        let c = ctx.resolve_r_value(&self.frac)?;

        let result = (self.op)(a,b, c);
        if result {
            if self.and_link {
                ctx.set_ra(ctx.instruction_pointer+1);
            }
            ctx.instruction_pointer = ctx.lookup(&self.target)?;
        } else {
            ctx.instruction_pointer += 1;
        }
        Ok(ctx)
    }
}

//

pub struct Yield { }

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
            Ok(a) => ParsedLine::OpCode(Box::new(a)),
            Err(e) => ParsedLine::Err(e),
        }
    }
}


//

pub enum ParsedLine
{
    OpCode(Box<dyn Instruction>),
    JumpLabel(String),
    Err(CompileError)
}

//

pub fn parse_one_line(line:&str) -> ParsedLine
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
            } else if "ls" == opcode || "lr" == opcode {
                ParsedLine::Err(CompileError { message: format!("{} unimplemented because I do not understand them yet", opcode)})
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

            } else if "bap" == opcode {
                BranchTernary::bap(parts).into()
            } else if "bapal" == opcode {
                BranchTernary::bapal(parts).into()

            } else if "bgt" == opcode {
                Branch::gt(parts).into()

            } else if "bdns" == opcode {
                BranchDevice::bdns(parts).into()
            } else if "bdnsal" == opcode {
                BranchDevice::bdnsal(parts).into()
            } else if "bdse" == opcode {
                BranchDevice::bdse(parts).into()
            } else if "bdseal" == opcode {
                BranchDevice::bdseal(parts).into()
            } else if "brdns" == opcode {
                BranchDevice::brdns(parts).into()
            } else if "brdse" == opcode {
                BranchDevice::brdse(parts).into()

            } else {
                ParsedLine::Err(CompileError{message: format!("unrecognized opcode {}", opcode)})
            }
        }

    };
}

//

pub struct DeviceStateBuilder
{
    rval : DeviceState,
}

impl DeviceStateBuilder
{
    pub fn new() -> DeviceStateBuilder
    {
        DeviceStateBuilder{rval:DeviceState::new()}
    }

    pub fn set(mut self, field:&str, value:f32) -> DeviceStateBuilder
    {
        self.rval .insert(field.to_string(), value);

        self
    }

    pub fn build(self) -> DeviceState
    {
        self.rval
    }
}

//

pub struct CompiledProgram
{
    codes: Vec<Box<dyn Instruction>>,
    labels: HashMap<String, InstructionPointer>,
}

impl CompiledProgram
{
    pub fn labels(&self) -> HashMap<String,InstructionPointer>
    {
        self.labels.clone()
    }

    pub fn get_instruction(&self, idx:InstructionPointer) -> Option<&dyn Instruction>
    {
        if (idx as usize) < self.codes.len() {
            Some(& * self.codes[idx as usize])
        } else {
            None
        }
    }
}


pub fn compile(src:&str) ->Result<CompiledProgram, CompileError>
{
    let lines:std::str::Lines = src.lines();

    compile_lines(lines)
}

pub fn compile_lines<'a,I>(lines: I) -> Result<CompiledProgram, CompileError>
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

pub fn execute_until_yields(program:&CompiledProgram, mut ctx:CPUContext, min_yields:u32) -> Result<CPUContext, ExecutionError>
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
            if yield_count >= min_yields {
                break;
            }
        }
        println!("IP = {}", ctx.instruction_pointer)
    }
    Ok(ctx)
}

pub fn execute_until_yields2<F>(program:&CompiledProgram, mut ctx:CPUContext, min_yields:u32, callback:F) -> Result<CPUContext, ExecutionError>
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
            if yield_count >= min_yields {
                break;
            }
        }
        println!("IP = {}", ctx.instruction_pointer)
    }
    Ok(ctx)
}
