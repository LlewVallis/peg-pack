use super::context::Context;
use super::input::Input;
use super::State;

pub trait Grammar: Sized {
    fn start_state(&self) -> State;

    unsafe fn dispatch_state<I: Input + ?Sized>(&self, state: State, ctx: &mut Context<I, Self>);
}
