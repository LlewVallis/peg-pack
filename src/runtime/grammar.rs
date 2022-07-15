use std::fmt::Debug;
use std::hash::Hash;

use super::context::Context;
use super::input::Input;
use super::State;

pub trait Grammar: Sized {
    type Label: Debug + Copy + Eq + Hash;

    fn start_state(&self) -> State;

    unsafe fn dispatch_state<I: Input + ?Sized>(&self, state: State, ctx: &mut Context<I, Self>);
}
