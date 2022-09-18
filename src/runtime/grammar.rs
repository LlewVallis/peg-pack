use std::fmt::Debug;
use std::hash::Hash;

use super::context::Context;
use super::input::Input;
use super::State;

pub trait Grammar: Sized {
    type Label: Label;
    type Expected: Expected<Self::Label>;

    fn start_state(&self) -> State;

    fn cache_slots(&self) -> usize;

    unsafe fn dispatch_state<I: Input + ?Sized>(&self, state: State, ctx: &mut Context<I, Self>);
}

pub trait Label: Debug + Copy + Eq + Hash {}

pub trait Expected<L: Label>: Debug + Copy + Eq + Hash {
    fn literals(&self) -> &[&[u8]];

    fn labels(&self) -> &[L];
}
