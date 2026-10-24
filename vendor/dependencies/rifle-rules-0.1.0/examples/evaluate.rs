use rifle_rules::{FactValue, Plan};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plan = Plan::from_json(include_bytes!("../fixtures/rust-program.json"))?;
    let outcome = plan.evaluate(&[
        FactValue::String("target"),
        FactValue::String("directory"),
        FactValue::Unavailable("access_denied"),
    ])?;
    println!("{:?}", outcome[0]);
    Ok(())
}
