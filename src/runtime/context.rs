use std::mem;
use std::mem::MaybeUninit;

use super::cache::Cache;
use super::grammar::Grammar;
use super::input::Input;
use super::result::Match;
use super::result::ParseResult;
use super::stack::Stack;
use super::{
    State, CACHE_WORK, CHOICE_WORK, LABEL_WORK, MARK_ERROR_WORK, MAX_UNCACHED_WORK, NOT_AHEAD_WORK,
    SEQ_WORK, SERIES_WORK,
};

#[allow(non_snake_case)]
fn FINISH_STATE<I: Input + ?Sized, G: Grammar>(_ctx: &mut Context<I, G>) {}

pub struct Context<'a, I: Input + ?Sized, G: Grammar> {
    input: &'a I,
    _grammar: &'a G,
    position: u32,
    state_stack: Stack<State<I, G>>,
    result_stack: Stack<MaybeUninit<ParseResult<G>>>,
    cache: Cache<G>,
}

impl<'a, I: Input + ?Sized, G: Grammar> Context<'a, I, G> {
    #[allow(unused)]
    pub fn run(input: &I, grammar: &G) -> ParseResult<G> {
        Context::new(input, grammar).finish()
    }

    fn finish(mut self) -> ParseResult<G> {
        unsafe {
            loop {
                let current_state = self.state();
                let finish_state: State<I, G> = FINISH_STATE::<I, G>;

                if mem::transmute::<_, fn()>(current_state)
                    == mem::transmute::<_, fn()>(finish_state)
                {
                    break;
                }

                current_state(&mut self);
            }

            self.take_result()
        }
    }

    fn new(input: &'a I, grammar: &'a G) -> Self {
        let mut states = Stack::<State<I, G>>::of(FINISH_STATE::<I, G>);
        states.push(grammar.start_state());

        Self {
            input,
            _grammar: grammar,
            position: 0,
            state_stack: states,
            result_stack: Stack::of(MaybeUninit::uninit()),
            cache: Cache::new(grammar),
        }
    }

    fn state(&self) -> State<I, G> {
        unsafe { *self.state_stack.top().unwrap_unchecked() }
    }

    fn state_mut(&mut self) -> &mut State<I, G> {
        unsafe { self.state_stack.top_mut().unwrap_unchecked() }
    }

    fn push_state(&mut self, state: State<I, G>) {
        self.state_stack.push(state);
    }

    unsafe fn pop_state(&mut self) {
        self.state_stack.pop();
    }

    unsafe fn result(&self) -> &ParseResult<G> {
        self.result_stack.top().unwrap_unchecked().assume_init_ref()
    }

    fn set_result(&mut self, result: ParseResult<G>) {
        unsafe {
            *self.result_stack.top_mut().unwrap_unchecked() = MaybeUninit::new(result);
        }
    }

    fn stash_result(&mut self) {
        self.result_stack.push(MaybeUninit::uninit());
    }

    unsafe fn pop_result(&mut self) -> ParseResult<G> {
        self.result_stack.pop().unwrap_unchecked().assume_init()
    }

    unsafe fn take_result(&mut self) -> ParseResult<G> {
        let top = self.result_stack.top_mut().unwrap_unchecked();
        mem::replace(top, MaybeUninit::uninit()).assume_init()
    }
}

#[allow(unused)]
impl<'a, I: Input + ?Sized, G: Grammar> Context<'a, I, G> {
    pub unsafe fn state_seq_start(&mut self, first: State<I, G>, continuation: State<I, G>) {
        *self.state_mut() = continuation;
        self.push_state(first);
    }

    pub unsafe fn state_seq_middle(&mut self, second: State<I, G>, continuation: State<I, G>) {
        if self.result().is_match() {
            self.stash_result();
            *self.state_mut() = continuation;
            self.push_state(second);
        } else {
            let result = self.take_result().add_work(SEQ_WORK);
            self.set_result(result);
            self.pop_state();
        }
    }

    pub unsafe fn state_seq_end(&mut self) {
        let second = self.pop_result();
        let first = self.take_result().unwrap_match_unchecked();

        match second {
            ParseResult::Matched(second) => {
                let result = Match::combine(first, second).add_work(SEQ_WORK);
                self.set_result(ParseResult::Matched(result));
            }
            ParseResult::Unmatched {
                scan_distance,
                work,
            } => {
                self.position -= first.distance();

                let scan_distance =
                    u32::max(first.scan_distance(), first.distance() + scan_distance);

                let work = work + first.work() + SEQ_WORK;
                self.set_result(ParseResult::Unmatched {
                    scan_distance,
                    work,
                })
            }
        }

        self.pop_state();
    }

    pub unsafe fn state_choice_start(&mut self, first: State<I, G>, continuation: State<I, G>) {
        *self.state_mut() = continuation;
        self.push_state(first);
    }

    pub unsafe fn state_choice_middle(&mut self, second: State<I, G>, continuation: State<I, G>) {
        if self.result().is_error_free() {
            let result = self.take_result().add_work(CHOICE_WORK);
            self.set_result(result);
            self.pop_state();
        } else {
            self.position -= self.result().distance();
            self.stash_result();
            *self.state_mut() = continuation;
            self.push_state(second);
        }
    }

    pub unsafe fn state_choice_end(&mut self) {
        let mut second = self.pop_result();
        let first = self.take_result();

        let work = first.work() + second.work() + CHOICE_WORK;

        if !first.is_match() {
            let result = second
                .extend_scan_distance(first.scan_distance())
                .with_work(work);
            self.set_result(result);
            self.pop_state();
            return;
        }

        let first = first.unwrap_match_unchecked();

        if !second.is_match() {
            self.position += first.distance();
            let result = first
                .extend_scan_distance(second.scan_distance())
                .with_work(work);
            self.set_result(ParseResult::Matched(result));
            self.pop_state();
            return;
        }

        let second = second.unwrap_match_unchecked();

        let first_dist = first.error_distance().unwrap_unchecked();
        let second_dist = second.error_distance();

        let use_second = match second_dist {
            Some(second_dist) => first_dist > second_dist,
            None => true,
        };

        if use_second {
            let result = second
                .extend_scan_distance(first.scan_distance())
                .with_work(work);
            self.set_result(ParseResult::Matched(result));
        } else {
            self.position -= second.distance();
            self.position += first.distance();
            let result = first
                .extend_scan_distance(second.scan_distance())
                .with_work(work);
            self.set_result(ParseResult::Matched(result));
        }

        self.pop_state();
    }

    pub unsafe fn state_first_choice_start(
        &mut self,
        first: State<I, G>,
        continuation: State<I, G>,
    ) {
        *self.state_mut() = continuation;
        self.push_state(first);
    }

    pub unsafe fn state_first_choice_middle(&mut self, second: State<I, G>) {
        let result = self.take_result();

        if result.is_match() {
            let result = result.add_work(CHOICE_WORK);
            self.set_result(result);
            self.pop_state();
        } else {
            self.position -= result.distance();
            *self.state_mut() = second;
        }
    }

    pub unsafe fn state_not_ahead_start(&mut self, target: State<I, G>, continuation: State<I, G>) {
        *self.state_mut() = continuation;
        self.push_state(target)
    }

    pub unsafe fn state_not_ahead_end(&mut self) {
        let result = self.take_result();
        self.position -= result.distance();
        let result = result.negate().add_work(NOT_AHEAD_WORK);
        self.set_result(result);

        self.pop_state();
    }

    pub unsafe fn state_error_start(&mut self, target: State<I, G>, continuation: State<I, G>) {
        *self.state_mut() = continuation;
        self.push_state(target);
    }

    pub unsafe fn state_error_end(&mut self, expected: G::Expected) {
        let result = self.take_result();
        let result = result.mark_error(expected).add_work(MARK_ERROR_WORK);
        self.set_result(result);
        self.pop_state();
    }

    pub unsafe fn state_label_start(&mut self, target: State<I, G>, continuation: State<I, G>) {
        *self.state_mut() = continuation;
        self.push_state(target);
    }

    pub unsafe fn state_label_end(&mut self, label: G::Label) {
        let result = self.take_result();
        let result = result.label(label).add_work(LABEL_WORK);
        self.set_result(result);
        self.pop_state();
    }

    pub unsafe fn state_cache_start(
        &mut self,
        slot: u32,
        target: State<I, G>,
        continuation: State<I, G>,
    ) {
        if let Some(result) = self.cache.get(slot, self.position) {
            self.position += result.distance();
            self.set_result(result);
            self.pop_state();
            return;
        }

        *self.state_mut() = continuation;
        self.push_state(target);
    }

    pub unsafe fn state_cache_end(&mut self, slot: u32) {
        if self.result().work() > MAX_UNCACHED_WORK {
            let result = self.take_result().with_work(CACHE_WORK);
            let position = self.position - result.distance();
            let result = self.cache.insert(slot, position, result);
            self.set_result(result);
        }

        self.pop_state();
    }

    pub unsafe fn state_delegate(&mut self, target: State<I, G>) {
        *self.state_mut() = target;
    }

    pub unsafe fn state_series(&mut self, matcher: impl FnOnce(&I, u32) -> (bool, u32)) {
        let (matched, length) = matcher(self.input, self.position);

        if matched {
            self.position += length;
            let result = Match::error_free(length, length, SERIES_WORK);
            self.set_result(ParseResult::Matched(result));
        } else {
            self.set_result(ParseResult::Unmatched {
                scan_distance: length,
                work: SERIES_WORK,
            })
        }

        self.pop_state();
    }
}
