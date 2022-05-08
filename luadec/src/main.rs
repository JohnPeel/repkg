use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use clap::Parser;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

mod parser {
    use std::fmt::Debug;

    use num_derive::{FromPrimitive, ToPrimitive};
    #[allow(unused_imports)]
    use num_traits::{FromPrimitive, ToPrimitive};

    use nom::{
        bytes::complete::take,
        combinator::{map_res, verify},
        multi::many_m_n,
        number::complete::{
            be_f32, be_f64, be_i16, be_i32, be_u16, be_u32, be_u64, le_f32, le_f64, le_i16, le_i32, le_u16, le_u32,
            le_u64, le_u8,
        },
        IResult,
    };

    type InfallibleResult<T> = Result<T, std::convert::Infallible>;

    #[derive(Debug, Clone, Copy)]
    pub struct Header<'a> {
        pub id_chunk: u8,
        pub signature: &'a str,
        pub version: u8,
        pub endianess: u8,
        pub sizeof_int: u8,
        pub sizeof_size_t: u8,
        pub sizeof_instruction: u8,
        pub size_instruction: u8,
        pub size_op: u8,
        pub size_b: u8,
        pub sizeof_number: u8,
        pub test_number: &'a [u8],
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Local<'a> {
        pub name: &'a str,
        pub start: i32,
        pub end: i32,
    }

    #[derive(Debug, Clone)]
    pub struct Constants<'a> {
        pub strings: Vec<&'a str>,
        pub numbers: Vec<f64>,
        pub functions: Vec<Function<'a>>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, FromPrimitive, ToPrimitive)]
    pub enum OpCode {
        End,
        Return,
        Call,
        TailCall,
        PushNil,
        Pop,
        PushInt,
        PushString,
        PushNumber,
        PushNegativeNumber,
        PushUpValue,
        GetLocal,
        GetGlobal,
        GetTable,
        GetDotted,
        GetIndexed,
        PushSelf,
        CreateTable,
        SetLocal,
        SetGlobal,
        SetTable,
        SetList,
        SetMap,
        Add,
        AddInt,
        Subtract,
        Multiply,
        Divide,
        Power,
        Concat,
        Minus,
        Not,
        JumpNotEqual,
        JumpEqual,
        JumpLessThan,
        JumpLessThanEqual,
        JumpGreaterThan,
        JumpGreaterThanEqual,
        JumpIfTrue,
        JumpIfFalse,
        JumpOnTrue,
        JumpOnFalse,
        Jump,
        PushNilJump,
        ForPrep,
        ForLoop,
        LForPrep,
        LForLoop,
        Closure,
    }

    #[allow(unused)]
    impl OpCode {
        pub fn is_jump(&self) -> bool {
            *self >= OpCode::JumpNotEqual && *self <= OpCode::Jump
        }
    }

    pub enum OpCodeMode {
        Unsigned,
        Signed,
        AB,
        None,
    }

    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    pub enum StackChange {
        Constant(u8),
        Delta,
        None,
    }

    impl Debug for StackChange {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Constant(u) => write!(f, "Constant({})", u),
                Self::Delta => write!(f, "Delta"),
                Self::None => write!(f, "None"),
            }
        }
    }

    impl OpCode {
        pub const fn mode(self) -> OpCodeMode {
            use OpCode::*;
            use OpCodeMode::*;
            match self {
                End => None,
                Return => Unsigned,
                Call | TailCall => AB,
                PushNil | Pop => Unsigned,
                PushInt => Signed,
                PushString | PushNumber | PushNegativeNumber | PushUpValue | GetLocal | GetGlobal => Unsigned,
                GetTable => None,
                GetDotted | GetIndexed | PushSelf | CreateTable | SetLocal | SetGlobal => Unsigned,
                SetTable | SetList => AB,
                SetMap => Unsigned,
                Add => None,
                AddInt => Signed,
                Subtract | Multiply | Divide | Power => None,
                Concat => Unsigned,
                Minus | Not => None,
                JumpNotEqual | JumpEqual | JumpLessThan | JumpLessThanEqual | JumpGreaterThan
                | JumpGreaterThanEqual | JumpIfTrue | JumpIfFalse | JumpOnTrue | JumpOnFalse | Jump => Signed,
                PushNilJump => None,
                ForPrep | ForLoop | LForPrep | LForLoop => Signed,
                Closure => AB,
            }
        }

        pub const fn push_count(self) -> StackChange {
            use OpCode::*;
            use StackChange::*;
            match self {
                End | Return => None,
                Call => Delta,
                TailCall => None,
                PushNil => Delta,
                Pop => None,
                PushInt | PushString | PushNumber | PushNegativeNumber | PushUpValue | GetLocal | GetGlobal
                | GetTable | GetDotted | GetIndexed => Constant(1),
                PushSelf => Constant(2),
                CreateTable => Constant(1),
                SetLocal | SetGlobal => None,
                SetTable | SetList | SetMap => None,
                Add | AddInt | Subtract | Multiply | Divide | Power => Constant(1),
                Concat => Constant(1),
                Minus | Not => Constant(1),
                JumpNotEqual | JumpEqual | JumpLessThan | JumpLessThanEqual | JumpGreaterThan
                | JumpGreaterThanEqual | JumpIfTrue | JumpIfFalse | JumpOnTrue | JumpOnFalse | Jump | PushNilJump
                | ForPrep | ForLoop => None,
                LForPrep => Constant(2),
                LForLoop => None,
                Closure => Constant(1),
            }
        }

        pub const fn pop_count(self) -> StackChange {
            use OpCode::*;
            use StackChange::*;
            match self {
                End => None,
                Return | Call | TailCall => Delta,
                PushNil => None,
                Pop => Delta,
                PushInt | PushString | PushNumber | PushNegativeNumber | PushUpValue | GetLocal | GetGlobal => None,
                GetTable => Constant(2),
                GetDotted | GetIndexed | PushSelf => Constant(1),
                CreateTable => None,
                SetLocal | SetGlobal => Constant(1),
                SetTable | SetList | SetMap => Delta,
                Add => Constant(2),
                AddInt => Constant(1),
                Subtract | Multiply | Divide | Power => Constant(2),
                Concat => Delta,
                Minus | Not => Constant(1),
                JumpNotEqual | JumpEqual | JumpLessThan | JumpLessThanEqual | JumpGreaterThan
                | JumpGreaterThanEqual => Constant(2),
                JumpIfTrue | JumpIfFalse | JumpOnTrue | JumpOnFalse => Constant(1),
                Jump => None,
                PushNilJump => None,
                ForPrep => None,
                ForLoop => Constant(3),
                LForPrep => None,
                LForLoop => Constant(3),
                Closure => Delta,
            }
        }
    }

    #[derive(Clone, Copy)]
    pub struct Instruction {
        instruction: usize,
        size_instruction: u8,
        size_op: u8,
        size_b: u8,
    }

    #[allow(unused)]
    impl Instruction {
        #[inline]
        pub fn op(&self) -> OpCode {
            FromPrimitive::from_usize(self.instruction & !((!0) << self.size_op)).expect("Invalid Instruction!")
        }

        #[inline]
        pub const fn u(&self) -> usize {
            self.instruction >> self.size_op
        }

        #[inline]
        pub const fn s(&self) -> isize {
            (self.u() as isize) - (((1 << (self.size_instruction - self.size_op)) - 1) >> 1)
        }

        #[inline]
        pub const fn a(&self) -> usize {
            self.instruction >> (self.size_op + self.size_b)
        }

        #[inline]
        pub const fn b(&self) -> usize {
            (self.instruction >> self.size_op) & !((!0) << self.size_b)
        }

        pub fn push_count(&self) -> usize {
            match self.op().push_count() {
                StackChange::Constant(r) => r as usize,
                StackChange::None => 0,
                StackChange::Delta => match self.op() {
                    OpCode::PushNil => self.u(),
                    OpCode::Call => self.b(),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
        }

        pub fn pop_count(&self) -> usize {
            match self.op().pop_count() {
                StackChange::Constant(r) => r as usize,
                StackChange::None => 0,
                StackChange::Delta => match self.op() {
                    OpCode::Pop => self.u(),
                    OpCode::SetTable => self.b(),
                    OpCode::SetList => todo!(),
                    OpCode::SetMap => todo!(),
                    OpCode::Concat => self.u(),
                    OpCode::Closure => self.b(),
                    OpCode::Call => self.a(),
                    OpCode::Return => self.u(),
                    _ => unreachable!(),
                },
            }
        }
    }

    impl Debug for Instruction {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let args = match self.op().mode() {
                OpCodeMode::Unsigned => format!("{}", self.u()),
                OpCodeMode::Signed => format!("{}", self.s()),
                OpCodeMode::AB => format!("{}, {}", self.a(), self.b()),
                OpCodeMode::None => "".to_string(),
            };

            write!(f, "{:?}({})", self.op(), args)
        }
    }

    #[derive(Debug, Clone)]
    pub struct Function<'a> {
        pub source: &'a str,
        pub line: i32,
        pub param_count: i32,
        pub is_vararg: bool,
        pub max_stack_size: i32,
        pub locals: Vec<Local<'a>>,
        pub lines: Vec<i32>,
        pub constants: Constants<'a>,
        pub code: Vec<Instruction>,
    }

    fn header(input: &[u8]) -> IResult<&[u8], Header<'_>> {
        let (input, id_chunk) = verify(le_u8, |x| *x == 0x1b)(input)?;
        let (input, signature) = verify(map_res(take(3usize), std::str::from_utf8), |x: &str| x == "Lua")(input)?;
        let (input, version) = verify(le_u8, |x| *x == 0x40)(input)?;
        let (input, endianess) = le_u8(input)?;
        let (input, sizeof_int) = le_u8(input)?;
        let (input, sizeof_size_t) = le_u8(input)?;
        let (input, sizeof_instruction) = le_u8(input)?;
        let (input, size_instruction) = le_u8(input)?;
        let (input, size_op) = le_u8(input)?;
        let (input, size_b) = le_u8(input)?;
        let (input, sizeof_number) = le_u8(input)?;
        let (input, test_number) = take(sizeof_number)(input)?;

        Ok((
            input,
            Header {
                id_chunk,
                signature,
                version,
                endianess,
                sizeof_int,
                sizeof_size_t,
                sizeof_instruction,
                size_instruction,
                size_op,
                size_b,
                sizeof_number,
                test_number,
            },
        ))
    }

    fn number<'a>(input: &'a [u8], header: Header<'a>) -> IResult<&'a [u8], f64> {
        match (header.sizeof_number, header.endianess) {
            (0x04, 0) => map_res(be_f32, |x| InfallibleResult::Ok(x as f64))(input),
            (0x04, 1) => map_res(le_f32, |x| InfallibleResult::Ok(x as f64))(input),
            (0x08, 0) => be_f64(input),
            (0x08, 1) => le_f64(input),
            _ => unimplemented!(),
        }
    }

    fn instruction<'a>(input: &'a [u8], header: Header<'a>) -> IResult<&'a [u8], Instruction> {
        let (input, instruction) = match (header.sizeof_instruction, header.endianess) {
            (0x02, 0) => map_res(be_u16, |x| InfallibleResult::Ok(x as u64))(input),
            (0x02, 1) => map_res(le_u16, |x| InfallibleResult::Ok(x as u64))(input),
            (0x04, 0) => map_res(be_u32, |x| InfallibleResult::Ok(x as u64))(input),
            (0x04, 1) => map_res(le_u32, |x| InfallibleResult::Ok(x as u64))(input),
            (0x08, 0) => be_u64(input),
            (0x08, 1) => le_u64(input),
            _ => unimplemented!(),
        }?;

        Ok((
            input,
            Instruction {
                instruction: instruction as usize,
                size_instruction: header.size_instruction,
                size_op: header.size_op,
                size_b: header.size_b,
            },
        ))
    }

    fn int<'a>(input: &'a [u8], header: Header<'a>) -> IResult<&'a [u8], i32> {
        match (header.sizeof_int, header.endianess) {
            (0x02, 0) => map_res(be_i16, |x| InfallibleResult::Ok(x as i32))(input),
            (0x02, 1) => map_res(le_i16, |x| InfallibleResult::Ok(x as i32))(input),
            (0x04, 0) => be_i32(input),
            (0x04, 1) => le_i32(input),
            _ => unimplemented!(),
        }
    }

    fn size_t<'a>(input: &'a [u8], header: Header<'a>) -> IResult<&'a [u8], usize> {
        match (header.sizeof_size_t, header.endianess) {
            (0x02, 0) => map_res(be_u16, |x| InfallibleResult::Ok(x as usize))(input),
            (0x02, 1) => map_res(le_u16, |x| InfallibleResult::Ok(x as usize))(input),
            (0x04, 0) => map_res(be_u32, |x| InfallibleResult::Ok(x as usize))(input),
            (0x04, 1) => map_res(le_u32, |x| InfallibleResult::Ok(x as usize))(input),
            (0x08, 0) => map_res(be_u64, |x| InfallibleResult::Ok(x as usize))(input),
            (0x08, 1) => map_res(le_u64, |x| InfallibleResult::Ok(x as usize))(input),
            _ => unimplemented!(),
        }
    }

    fn string<'a>(input: &'a [u8], header: Header<'a>) -> IResult<&'a [u8], &'a str> {
        let (input, length) = size_t(input, header)?;
        let (input, str) = map_res(take(length), std::str::from_utf8)(input)?;
        Ok((input, if length > 0 { &str[..str.len() - 1] } else { str }))
    }

    fn local<'a>(input: &'a [u8], header: Header<'a>) -> IResult<&'a [u8], Local<'a>> {
        let (input, name) = string(input, header)?;
        let (input, start) = int(input, header)?;
        let (input, end) = int(input, header)?;
        Ok((input, Local { name, start, end }))
    }

    fn locals<'a>(input: &'a [u8], header: Header<'a>) -> IResult<&'a [u8], Vec<Local<'a>>> {
        let (input, count) = int(input, header)?;
        many_m_n(count as usize, count as usize, |input| local(input, header))(input)
    }

    fn lines<'a>(input: &'a [u8], header: Header<'a>) -> IResult<&'a [u8], Vec<i32>> {
        let (input, count) = int(input, header)?;
        many_m_n(count as usize, count as usize, |input| int(input, header))(input)
    }

    fn constants<'a>(input: &'a [u8], header: Header<'a>) -> IResult<&'a [u8], Constants<'a>> {
        let (input, count) = int(input, header)?;
        let (input, strings) = many_m_n(count as usize, count as usize, |input| string(input, header))(input)?;
        let (input, count) = int(input, header)?;
        let (input, numbers) = many_m_n(count as usize, count as usize, |input| number(input, header))(input)?;
        let (input, count) = int(input, header)?;
        let (input, functions) = many_m_n(count as usize, count as usize, |input| function(input, header))(input)?;

        Ok((
            input,
            Constants {
                strings,
                numbers,
                functions,
            },
        ))
    }

    fn code<'a>(input: &'a [u8], header: Header<'a>) -> IResult<&'a [u8], Vec<Instruction>> {
        let (input, count) = int(input, header)?;
        let (input, code) = many_m_n(count as usize, count as usize, |input| instruction(input, header))(input)?;
        assert!(code[code.len() - 1].op() == OpCode::End);
        Ok((input, code))
    }

    fn function<'a>(input: &'a [u8], header: Header<'a>) -> IResult<&'a [u8], Function<'a>> {
        let (input, source) = string(input, header)?;
        let (input, line) = int(input, header)?;
        let (input, param_count) = int(input, header)?;
        let (input, is_vararg) = map_res(le_u8, |x| InfallibleResult::Ok(x == 1))(input)?;
        let (input, max_stack_size) = int(input, header)?;

        let (input, locals) = locals(input, header)?;
        let (input, lines) = lines(input, header)?;
        let (input, constants) = constants(input, header)?;
        let (input, code) = code(input, header)?;

        Ok((
            input,
            Function {
                source,
                line,
                param_count,
                is_vararg,
                max_stack_size,
                locals,
                lines,
                constants,
                code,
            },
        ))
    }

    pub fn lua(input: &[u8]) -> IResult<&[u8], (Header<'_>, Function<'_>)> {
        let (input, header) = header(input)?;
        let (input, function) = function(input, header)?;

        assert_eq!(0, input.len());

        Ok((input, (header, function)))
    }
}

mod code_generation {
    use std::{collections::VecDeque, fmt::Debug};

    use super::parser::*;

    #[derive(Clone)]
    pub struct Node {
        instruction: Instruction,
        children: Vec<Node>,
    }

    impl Debug for Node {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if !self.children.is_empty() {
                write!(f, "Node({:?}, {:#?})", self.instruction, self.children)
            } else {
                write!(f, "Node({:?})", self.instruction)
            }
        }
    }

    impl Node {
        #[allow(unused)]
        pub fn instruction_count(&self) -> usize {
            self.children.iter().map(|node| node.instruction_count()).sum::<usize>() + 1
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    pub fn to_nodes(instructions: Vec<Instruction>, constants: &Constants) -> Vec<Node> {
        let mut queue: VecDeque<Instruction> = instructions.into_iter().rev().collect();
        let mut unused: VecDeque<Node> = VecDeque::new();
        let mut terminated = Vec::new();

        while !queue.is_empty() {
            let instruction = queue.pop_back().unwrap();
            log::info!(
                "{: <30?} {} {} {:?}",
                instruction,
                instruction.pop_count(),
                instruction.push_count(),
                unused.iter().map(|node| node.instruction).collect::<Vec<Instruction>>()
            );

            let push_count = instruction.push_count();
            let pop_count = instruction.pop_count();

            let mut children = Vec::new();
            let mut needed = pop_count;

            while needed > 0 {
                let next_unused = unused.pop_back().unwrap();
                needed -= next_unused.instruction.push_count();
                children.push(next_unused);
            }

            if instruction.op().is_jump() && instruction.s() > 0 {
                let jump: Vec<Instruction> = queue
                    .split_off(queue.len() - instruction.s() as usize)
                    .into_iter()
                    .rev()
                    .collect();
                children.extend(to_nodes(jump, constants).into_iter());
            }

            let node = Node { instruction, children };

            if push_count != 0 {
                unused.push_back(node);
            } else {
                terminated.push(node);
            }
        }

        assert_eq!(0, unused.len());
        terminated
    }

    #[allow(unused)]
    pub fn process_node(node: &Node, locals: &mut Vec<Local>, constants: &Constants) -> String {
        let children: Vec<String> = node
            .children
            .iter()
            .map(|node| process_node(node, locals, constants))
            .collect();
        let instruction = node.instruction;

        use OpCode::*;
        match instruction.op() {
            End => "".to_string(),
            Return => format!("return {}", children.into_iter().collect::<Vec<String>>().join(", ")),
            Call => {
                let mut args = Vec::new();
                for i in 0..children.len() - 1 {
                    args.push(children.get(i).unwrap().to_owned());
                }
                format!("{}({})", children.last().unwrap(), args.join(", "))
            }
            //TailCall
            PushNil => (0..instruction.u()).map(|_| "nil".to_owned()).collect::<String>(),
            //Pop
            PushInt => instruction.s().to_string(),
            PushString => format!("\"{}\"", constants.strings.get(instruction.u()).unwrap()),
            PushNumber => constants.numbers.get(instruction.u()).unwrap().to_string(),
            PushNegativeNumber => (-constants.numbers.get(instruction.u()).unwrap()).to_string(),
            //PushUpValue
            GetLocal => locals
                .get(instruction.u())
                .map(|l| l.name.to_string())
                .unwrap_or(format!("local_{}", instruction.u())),
            GetGlobal => constants.strings.get(instruction.u()).unwrap().to_string(),
            //GetTable
            GetDotted => format!(
                "{}.{}",
                children.get(0).unwrap(),
                constants.strings.get(instruction.u()).unwrap()
            ),
            //GetIndexed
            PushSelf => format!(
                "{}:{}",
                children.get(0).unwrap(),
                constants.strings.get(instruction.u()).unwrap()
            ),
            CreateTable => {
                if instruction.u() > 0 {
                    format!("{{n={}}}", instruction.u())
                } else {
                    "{}".to_string()
                }
            }
            //SetLocal,
            SetGlobal => format!(
                "{} = {}",
                constants.strings.get(instruction.u()).unwrap(),
                children.get(0).unwrap()
            ),
            SetTable => format!(
                "{}[{}] = {}",
                children.get(2).unwrap(),
                children.get(1).unwrap(),
                children.get(0).unwrap()
            ),
            //SetList,
            //SetMap,
            //Add,
            AddInt => format!("{} + {}", children.get(0).unwrap(), instruction.s()),
            //Subtract,
            //Multiply,
            //Divide,
            //Power,
            //Concat,
            //Minus,
            //Not,
            op if op >= JumpNotEqual && op <= JumpGreaterThanEqual => {
                let op = match op {
                    JumpNotEqual => "==",
                    JumpEqual => "~=",
                    JumpLessThan => ">=",
                    JumpLessThanEqual => ">",
                    JumpGreaterThan => "<=",
                    JumpGreaterThanEqual => "<",
                    _ => unreachable!(),
                };
                let (params, body) = children.split_at(2);
                let body: Vec<&str> = body.iter().flat_map(|line| line.split('\n')).collect();
                format!(
                    "if ({} {} {}) then\n  {}\nend",
                    params[1],
                    op,
                    params[0],
                    body.join("\n  ")
                )
            }
            op if op >= JumpIfTrue && op <= JumpIfFalse => {
                let op = if op == JumpIfTrue { "not " } else { "" };
                let (params, body) = children.split_at(1);
                let body: Vec<&str> = body.iter().flat_map(|line| line.split('\n')).collect();
                format!("if ({} {}) then\n  {}\nend", op, params[0], body.join("\n  "))
            }

            //JumpOnTrue,
            //JumpOnFalse,
            //Jump,
            //PushNilJump,
            //ForPrep,
            //ForLoop,
            //LForPrep,
            //LForLoop,
            Closure => {
                let mut args = Vec::new();
                let function = constants.functions.get(instruction.a()).unwrap();
                for i in 0..function.param_count {
                    args.push(format!("local_{}", i));
                }
                format!("function({})\n{}\nend", args.join(", "), children.join("\n"))
            }
            _ => todo!("{:?} ({:?})", instruction, children),
        }
    }
}

#[derive(Parser)]
#[clap(author, version, about = None, long_about = None)]
struct Opts {
    #[clap(parse(from_os_str))]
    input: PathBuf,
}

fn main() -> Result<(), BoxError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .default_format()
        .parse_default_env()
        .init();

    let opts: Opts = Opts::parse();

    let input = {
        let mut reader = BufReader::new(File::open(opts.input)?);
        let mut input = Vec::new();
        reader.read_to_end(&mut input)?;
        input
    };

    let (_, (_header, function)) = parser::lua(&input).map_err(|err| -> BoxError { format!("{:#?}", err).into() })?;

    log::info!("\n{:#?}", function);

    let nodes = code_generation::to_nodes(function.code.clone(), &function.constants);
    log::info!("AST Tree\n{:#?}", nodes);

    let code: Vec<String> = nodes
        .into_iter()
        .map(|node| code_generation::process_node(&node, &mut vec![], &function.constants.clone()))
        .collect();
    log::info!("Generated Code\n{}", code.join("\n"));

    Ok(())
}
