use rust_fsm::*;

state_machine! {
    #[derive(Debug)] #[repr(C)] Door => #[derive(Debug)] #[repr(C)] Action => __

    Open => Key => Closed,
    Closed => Key => Open,
    Open => Break => Broken,
    Closed => Break => Broken,
}

#[test]
fn simple() {
    let mut machine = Door::Open;
    machine.consume(Action::Key).unwrap();
    println!("{machine:?}");
    machine.consume(Action::Key).unwrap();
    println!("{machine:?}");
    machine.consume(Action::Break).unwrap();
    println!("{machine:?}");
}
