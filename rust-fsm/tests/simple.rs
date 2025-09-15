use rust_fsm::*;

state_machine! {
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    door(Open)

    Open => Key => Closed,
    Closed => Key => Open,
    Open => Break => Broken,
    Closed => Break => Broken,
    Open => Thing(u32 => 5) => Fine,
    // Open(u32) => Key => Open [output],
    // Open(u32) => {
    //    Key => Open,
    // }
}

#[test]
fn simple() {
    let mut machine = door::StateMachine::new();
    machine.consume(door::Input::Key).unwrap();
    println!("{:?}", machine.state());
    machine.consume(door::Input::Key).unwrap();
    println!("{:?}", machine.state());
    machine.consume(door::Input::Break).unwrap();
    println!("{:?}", machine.state());
}
