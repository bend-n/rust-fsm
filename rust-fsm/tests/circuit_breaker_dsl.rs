/// A dummy implementation of the Circuit Breaker pattern to demonstrate
/// capabilities of its library DSL for defining finite state machines.
/// https://martinfowler.com/bliki/CircuitBreaker.html
use rust_fsm::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

state_machine! {
    /// A dummy implementation of the Circuit Breaker pattern to demonstrate
    /// capabilities of its library DSL for defining finite state machines.
    /// https://martinfowler.com/bliki/CircuitBreaker.html
    #[derive(Debug)]
    pub CircuitBreaker =>
    #[derive(Debug)] pub Result =>
    #[derive(Debug)] pub Action

    Closed => Unsuccessful => Open [SetupTimer],
    Open => TimerTriggered => HalfOpen,
    HalfOpen => {
        Successful => Closed,
        Unsuccessful => Open [SetupTimer]
    }
}

#[test]
fn circit_breaker_dsl() {
    let machine = CircuitBreaker::Closed;

    // Unsuccessful request
    let machine = Arc::new(Mutex::new(machine));
    {
        let mut lock = machine.lock().unwrap();
        let res = lock.consume(Result::Unsuccessful).unwrap();
        assert!(matches!(res, Some(Action::SetupTimer)));
        assert!(matches!(*lock, CircuitBreaker::Open));
    }

    // Set up a timer
    let machine_wait = machine.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::new(5, 0));
        let mut lock = machine_wait.lock().unwrap();
        let res = lock.consume(Result::TimerTriggered).unwrap();
        assert!(matches!(res, None));
        assert!(matches!(*lock, CircuitBreaker::HalfOpen));
    });

    // Try to pass a request when the circuit breaker is still open
    let machine_try = machine.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::new(1, 0));
        let mut lock = machine_try.lock().unwrap();
        let res = lock.consume(Result::Successful);
        assert!(matches!(res, Err(_)));
        assert!(matches!(*lock, CircuitBreaker::Open));
    });

    // Test if the circit breaker was actually closed
    std::thread::sleep(Duration::new(7, 0));
    {
        let mut lock = machine.lock().unwrap();
        let res = lock.consume(Result::Successful).unwrap();
        assert!(matches!(res, None));
        assert!(matches!(*lock, CircuitBreaker::Closed));
    }
}
