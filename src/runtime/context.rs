use std::mem;
use std::mem::MaybeUninit;

use super::grammar::Grammar;
use super::input::Input;
use super::result::ParseResult;
use super::{State, FINISH_STATE};

#[allow(unused)]
pub fn match_length<I: Input + ?Sized>(input: &I, grammar: &impl Grammar) -> Option<usize> {
    let result = Context::run(input, grammar);
    result.match_length()
}

pub struct Context<'a, I: Input + ?Sized, G: Grammar> {
    input: &'a I,
    grammar: &'a G,
    position: usize,
    state_stack: Stack<State>,
    result_stack: Stack<MaybeUninit<ParseResult>>,
}

impl<'a, I: Input + ?Sized, G: Grammar> Context<'a, I, G> {
    fn run(input: &I, grammar: &G) -> ParseResult {
        Context::new(input, grammar).finish()
    }

    fn finish(mut self) -> ParseResult {
        unsafe {
            while self.state() != FINISH_STATE {
                self.grammar.dispatch_state(self.state(), &mut self);
            }

            self.take_result()
        }
    }

    fn new(input: &'a I, grammar: &'a G) -> Self {
        let mut states = Stack::of(FINISH_STATE);
        states.push(grammar.start_state());

        Self {
            input,
            grammar,
            position: 0,
            state_stack: states,
            result_stack: Stack::of(MaybeUninit::uninit()),
        }
    }

    unsafe fn peek(&self) -> Option<u8> {
        self.input.get(self.position)
    }

    unsafe fn state(&self) -> State {
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
impl<'a, I: Input + ?Sized, G: Grammar> Context<'a, I, G> {
    pub unsafe fn state_seq_start<const FIRST: State, const CONTINUATION: State>(&mut self) {
        *self.state_mut() = CONTINUATION;
        self.push_state(FIRST);
    }

    pub unsafe fn state_seq_middle<const SECOND: State, const CONTINUATION: State>(&mut self) {
        if self.result().has_matched() {
            self.stash_result();
            *self.state_mut() = CONTINUATION;
            self.push_state(SECOND);
        } else {
            self.pop_state();
        }
    }

    pub unsafe fn state_seq_end(&mut self) {
        let second = self.pop_result();
        let first = self.take_result();

        if second.has_matched() {
            let result = ParseResult::combine_matches(first, second);
            self.set_result(result);
        } else {
            self.position -= first.unwrap_distance_unchecked();
            let scan_distance = ParseResult::combined_scan_distance(&first, &second);
            let result = ParseResult::unmatched(scan_distance);
            self.set_result(result);
        }

        self.pop_state();
    }

    pub unsafe fn state_choice_start<const FIRST: State, const CONTINUATION: State>(&mut self) {
        *self.state_mut() = CONTINUATION;
        self.push_state(FIRST);
    }

    pub unsafe fn state_choice_middle<const SECOND: State, const CONTINUATION: State>(&mut self) {
        if self.result().is_error_free() {
            self.pop_state();
        } else {
            self.position -= self.result().distance();
            self.stash_result();
            *self.state_mut() = CONTINUATION;
            self.push_state(SECOND);
        }
    }

    pub unsafe fn state_choice_end(&mut self) {
        let mut second = self.pop_result();
        let first = self.take_result();

        if !first.has_matched() {
            let result = second.max_scan_distance(first.scan_distance());
            self.set_result(result);
            self.pop_state();
            return;
        }

        if !second.has_matched() {
            self.position += first.unwrap_distance_unchecked();
            let result = first.max_scan_distance(second.scan_distance());
            self.set_result(result);
            self.pop_state();
            return;
        }

        let first_dist = first.unwrap_error_distance_unchecked().unwrap_unchecked();
        let second_dist = second.unwrap_error_distance_unchecked();

        let use_second = match second_dist {
            Some(second_dist) => first_dist > second_dist,
            None => true,
        };

        if use_second {
            let result = second.max_scan_distance(first.scan_distance());
            self.set_result(result);
        } else {
            self.position -= second.unwrap_distance_unchecked();
            self.position += first.unwrap_distance_unchecked();
            let result = first.max_scan_distance(second.scan_distance());
            self.set_result(result);
        }

        self.pop_state();
    }

    pub unsafe fn state_not_ahead_start<const TARGET: State, const CONTINUATION: State>(&mut self) {
        *self.state_mut() = CONTINUATION;
        self.push_state(TARGET)
    }

    pub unsafe fn state_not_ahead_end(&mut self) {
        let result = self.take_result();
        self.position -= result.distance();
        self.set_result(result.negate());

        self.pop_state();
    }

    pub unsafe fn state_error_start<const TARGET: State, const CONTINUATION: State>(&mut self) {
        *self.state_mut() = CONTINUATION;
        self.push_state(TARGET);
    }

    pub unsafe fn state_error_end(&mut self) {
        let result = self.take_result();
        self.set_result(result.mark_error());
        self.pop_state();
    }

    pub unsafe fn state_delegate<const TARGET: State>(&mut self) {
        *self.state_mut() = TARGET;
    }

    pub unsafe fn state_class(&mut self, in_class: impl FnOnce(u8) -> bool) {
        if let Some(char) = self.peek() {
            if in_class(char) {
                self.position += 1;
                self.set_result(ParseResult::error_free(1, 1));
                self.pop_state();
                return;
            }
        }

        self.set_result(ParseResult::unmatched(1));
        self.pop_state();
    }

    pub unsafe fn state_empty(&mut self) {
        self.set_result(ParseResult::empty(0));
        self.pop_state();
    }
}
