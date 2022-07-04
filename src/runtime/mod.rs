//! Runtime common to all generated parsers. Copied into the build directory
//! when generating a parser

use std::hint::unreachable_unchecked;
use std::mem;
use std::mem::MaybeUninit;

pub type Char = u8;

#[inline(always)]
fn likely(condition: bool) -> bool {
    let divisor = if condition { 1 } else { 0 };

    if (1i32).checked_div(divisor).is_some() {
        true
    } else {
        false
    }
}

pub trait Implementation {
    fn start(&self) -> State;

    unsafe fn dispatch(&mut self, ctx: &mut Context);
}

#[allow(unused)]
macro_rules! generate_implementation {
    ($start:expr, $dispatcher:expr) => {
        struct Impl;

        impl Implementation for Impl {
            fn start(&self) -> State {
                $start
            }

            unsafe fn dispatch(&mut self, ctx: &mut Context) {
                ($dispatcher)(ctx)
            }
        }
    };
}

#[allow(unused)]
macro_rules! generate_main {
    ($impl:ident) => {
        pub fn main() {
            use std::io;
            use std::io::Read;
            use std::time::Instant;

            let mut buffer = Vec::new();
            io::stdin()
                .read_to_end(&mut buffer)
                .expect("could not read input");

            let start = Instant::now();
            let parser = Parser::new(&buffer, $impl);
            let elapsed = start.elapsed();

            if parser.matched() {
                println!(
                    "Matched {} characters in {:.1?}",
                    parser.match_length(),
                    elapsed
                );
            } else {
                println!("Did not match in {:.1?}", elapsed);
            }
        }
    };
}

#[allow(unused)]
pub struct Parser {
    result: ParseResult,
}

impl Parser {
    #[allow(unused)]
    pub fn new(input: &[Char], implementation: impl Implementation) -> Self {
        let input = Input { buffer: input };
        let result = Context::run(input, implementation);
        Self { result }
    }

    #[allow(unused)]
    pub fn matched(&self) -> bool {
        match self.result {
            ParseResult::Matched(_) => true,
            ParseResult::Unmatched => false,
        }
    }

    #[allow(unused)]
    pub fn match_length(&self) -> usize {
        match &self.result {
            ParseResult::Matched(node) => node.length,
            ParseResult::Unmatched => 0,
        }
    }
}

struct Input<'a> {
    buffer: &'a [Char],
}

impl<'a> Input<'a> {
    fn get(&'a self, index: usize) -> Option<Char> {
        let result = self.buffer.get(index);
        if likely(result.is_some()) {
            unsafe { Some(*result.unwrap_unchecked()) }
        } else {
            None
        }
    }
}

pub type State = u32;

const FINISH_STATE: State = 0;

pub struct Context<'a> {
    input: Input<'a>,
    position: usize,
    state_stack: Stack<State>,
    result_stack: Stack<MaybeUninit<ParseResult>>,
}

impl<'a> Context<'a> {
    fn run(input: Input<'a>, mut implementation: impl Implementation) -> ParseResult {
        unsafe {
            let ctx = Self::new(input, implementation.start());
            ctx.finish(|ctx| implementation.dispatch(ctx))
        }
    }

    fn new(input: Input<'a>, start: State) -> Self {
        let mut states = Stack::of(FINISH_STATE);
        states.push(start);

        Self {
            input,
            position: 0,
            state_stack: states,
            result_stack: Stack::of(MaybeUninit::uninit()),
        }
    }

    unsafe fn finish(mut self, mut dispatch: impl FnMut(&mut Context)) -> ParseResult {
        while self.state() != FINISH_STATE {
            dispatch(&mut self);
        }

        self.take_result()
    }

    unsafe fn peek(&self) -> Option<Char> {
        self.input.get(self.position)
    }

    pub unsafe fn state(&self) -> State {
        *self.state_stack.top()
    }

    unsafe fn state_mut(&mut self) -> &mut State {
        self.state_stack.top_mut()
    }

    unsafe fn push_state(&mut self, state: State) {
        self.state_stack.push(state);
    }

    unsafe fn pop_state(&mut self) {
        self.state_stack.pop();
    }

    unsafe fn result(&self) -> &ParseResult {
        self.result_stack.top().assume_init_ref()
    }

    unsafe fn set_result(&mut self, result: ParseResult) {
        *self.result_stack.top_mut() = MaybeUninit::new(result);
    }

    fn stash_result(&mut self) {
        self.result_stack.push(MaybeUninit::uninit());
    }

    unsafe fn pop_result(&mut self) -> ParseResult {
        self.result_stack.pop().assume_init()
    }

    unsafe fn take_result(&mut self) -> ParseResult {
        let top = self.result_stack.top_mut();
        mem::replace(top, MaybeUninit::uninit()).assume_init()
    }
}

impl<'a> Drop for Context<'a> {
    fn drop(&mut self) {
        for result in &mut self.result_stack.elements {
            unsafe {
                result.assume_init_drop();
            }
        }
    }
}

pub enum ParseResult {
    Matched(Node),
    Unmatched,
}

impl ParseResult {
    unsafe fn unwrap_node_unchecked(self) -> Node {
        match self {
            ParseResult::Matched(node) => node,
            ParseResult::Unmatched => unreachable_unchecked(),
        }
    }

    unsafe fn unwrap_length_unchecked(&self) -> usize {
        match self {
            ParseResult::Matched(node) => node.length,
            ParseResult::Unmatched => unreachable_unchecked(),
        }
    }

    unsafe fn unwrap_error_distance_unchecked(&self) -> Option<usize> {
        match self {
            ParseResult::Matched(node) => node.error_distance,
            ParseResult::Unmatched => unreachable_unchecked(),
        }
    }
}

pub struct Node {
    length: usize,
    error_distance: Option<usize>,
}

impl Node {
    fn empty() -> Self {
        Self {
            length: 0,
            error_distance: None,
        }
    }

    fn error_free(length: usize) -> Self {
        Self {
            length,
            error_distance: None,
        }
    }

    fn combine(left: Self, right: Self) -> Self {
        let length = left.length + right.length;

        let error_distance = left
            .error_distance
            .or_else(|| right.error_distance.map(|distance| distance + left.length));

        Self {
            length,
            error_distance,
        }
    }

    fn expected(self) -> Self {
        Self {
            length: self.length,
            error_distance: Some(0),
        }
    }
}

struct Stack<T> {
    top: T,
    elements: Vec<T>,
}

impl<T> Stack<T> {
    fn of(value: T) -> Self {
        Self {
            top: value,
            elements: Vec::new(),
        }
    }

    unsafe fn top(&self) -> &T {
        &self.top
    }

    unsafe fn top_mut(&mut self) -> &mut T {
        &mut self.top
    }

    fn push(&mut self, value: T) {
        unsafe {
            let old_top = mem::replace(self.top_mut(), value);
            self.elements.push(old_top);
        }
    }

    unsafe fn pop(&mut self) -> T {
        let next = self.elements.pop().unwrap_unchecked();
        mem::replace(self.top_mut(), next)
    }
}

#[allow(unused)]
pub unsafe fn state_seq_start(ctx: &mut Context, first: State, continuation: State) {
    *ctx.state_mut() = continuation;
    ctx.push_state(first);
}

#[allow(unused)]
pub unsafe fn state_seq_middle(ctx: &mut Context, second: State, continuation: State) {
    match ctx.result() {
        ParseResult::Matched(_) => {
            ctx.stash_result();
            *ctx.state_mut() = continuation;
            ctx.push_state(second);
        }
        ParseResult::Unmatched => {
            ctx.pop_state();
        }
    }
}

#[allow(unused)]
pub unsafe fn state_seq_end(ctx: &mut Context) {
    match ctx.result() {
        ParseResult::Matched(_) => {
            let second = ctx.pop_result().unwrap_node_unchecked();
            let first = ctx.take_result().unwrap_node_unchecked();
            ctx.set_result(ParseResult::Matched(Node::combine(first, second)));
        }
        ParseResult::Unmatched => {
            ctx.pop_result();
            ctx.position -= ctx.result().unwrap_length_unchecked();
            ctx.set_result(ParseResult::Unmatched);
        }
    }

    ctx.pop_state();
}

#[allow(unused)]
pub unsafe fn state_choice_start(ctx: &mut Context, first: State, continuation: State) {
    *ctx.state_mut() = continuation;
    ctx.push_state(first);
}

#[allow(unused)]
pub unsafe fn state_choice_middle(ctx: &mut Context, second: State, continuation: State) {
    match ctx.result() {
        ParseResult::Matched(node) => {
            if node.error_distance.is_none() {
                ctx.pop_state();
            } else {
                ctx.position -= node.length;
                ctx.stash_result();
                *ctx.state_mut() = continuation;
            }
        }
        ParseResult::Unmatched => {
            *ctx.state_mut() = second;
        }
    }
}

#[allow(unused)]
pub unsafe fn state_choice_end(ctx: &mut Context) {
    let second = ctx.pop_result();
    let first = ctx.result();

    if let ParseResult::Unmatched = second {
        ctx.position += first.unwrap_length_unchecked();
        ctx.pop_state();
        return;
    }

    let first_dist = first.unwrap_error_distance_unchecked().unwrap_unchecked();
    let second_dist = second.unwrap_error_distance_unchecked();

    let use_second = match second_dist {
        Some(second_dist) => first_dist > second_dist,
        None => true,
    };

    if use_second {
        ctx.set_result(second);
    } else {
        ctx.position =
            ctx.position - second.unwrap_length_unchecked() + first.unwrap_length_unchecked();
    }

    ctx.pop_state();
}

#[allow(unused)]
pub unsafe fn state_not_ahead_start(ctx: &mut Context, target: State, continuation: State) {
    *ctx.state_mut() = continuation;
    ctx.push_state(target)
}

#[allow(unused)]
pub unsafe fn state_not_ahead_end(ctx: &mut Context) {
    match ctx.result() {
        ParseResult::Matched(node) => {
            ctx.position -= node.length;
            ctx.set_result(ParseResult::Unmatched);
        }
        ParseResult::Unmatched => {
            ctx.set_result(ParseResult::Matched(Node::empty()));
        }
    }

    ctx.pop_state();
}

#[allow(unused)]
pub unsafe fn state_error_start(ctx: &mut Context, target: State, continuation: State) {
    *ctx.state_mut() = continuation;
    ctx.push_state(target);
}

#[allow(unused)]
pub unsafe fn state_error_end(ctx: &mut Context) {
    let new_result = match ctx.take_result() {
        ParseResult::Matched(node) => ParseResult::Matched(node.expected()),
        ParseResult::Unmatched => ParseResult::Unmatched,
    };

    ctx.set_result(new_result);
    ctx.pop_state();
}

#[allow(unused)]
pub unsafe fn state_delegate(ctx: &mut Context, target: State) {
    *ctx.state_mut() = target;
}

#[allow(unused)]
pub unsafe fn state_class(ctx: &mut Context, in_class: impl FnOnce(Char) -> bool) {
    if let Some(char) = ctx.peek() {
        if in_class(char) {
            ctx.position += 1;
            ctx.set_result(ParseResult::Matched(Node::error_free(1)));
            ctx.pop_state();
            return;
        }
    }

    ctx.set_result(ParseResult::Unmatched);
    ctx.pop_state();
}

#[allow(unused)]
pub unsafe fn state_empty(ctx: &mut Context) {
    ctx.set_result(ParseResult::Matched(Node::empty()));
    ctx.pop_state();
}
