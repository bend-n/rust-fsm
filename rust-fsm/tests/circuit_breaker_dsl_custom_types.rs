/// A dummy implementation of the Circuit Breaker pattern to demonstrate
/// capabilities of its library DSL for defining finite state machines.
/// https://martinfowler.com/bliki/CircuitBreaker.html
use rust_fsm::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum Input {
    Successful,
    Unsuccessful,
    TimerTriggered,
}

#[derive(Clone, Copy)]
pub enum State {
    Closed,
    HalfOpen,
    Open,
}

#[derive(Clone, Copy)]
pub enum Output {
    SetupTimer,
}

state_machine! {
    crate::State: Closed => crate::Input => crate::Output

    Closed => Unsuccessful => Open [SetupTimer],
    Open => TimerTriggered => HalfOpen,
    HalfOpen => {
        Successful => Closed,
        Unsuccessful => Open [SetupTimer]
    }
}

#[test]
fn circit_breaker_dsl() {
    let machine = State::new();

    // Unsuccessful request
    let machine = Arc::new(Mutex::new(machine));
    {
        let mut lock = machine.lock().unwrap();
        let res = lock.consume(Input::Unsuccessful).unwrap();
        assert!(matches!(res, Some(Output::SetupTimer)));
        assert!(matches!(*lock, State::Open));
    }

    // Set up a timer
    let machine_wait = machine.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::new(5, 0));
        let mut lock = machine_wait.lock().unwrap();
        let res = lock.consume(Input::TimerTriggered).unwrap();
        assert!(matches!(res, None));
        assert!(matches!(*lock, State::HalfOpen));
    });

    // Try to pass a request when the circuit breaker is still open
    let machine_try = machine.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::new(1, 0));
        let mut lock = machine_try.lock().unwrap();
        let res = lock.consume(Input::Successful);
        assert!(matches!(res, Err(TransitionImpossibleError)));
        assert!(matches!(*lock, State::Open));
    });

    // Test if the circit breaker was actually closed
    std::thread::sleep(Duration::new(7, 0));
    {
        let mut lock = machine.lock().unwrap();
        let res = lock.consume(Input::Successful).unwrap();
        assert!(matches!(res, None));
        assert!(matches!(*lock, State::Closed));
    }
}
