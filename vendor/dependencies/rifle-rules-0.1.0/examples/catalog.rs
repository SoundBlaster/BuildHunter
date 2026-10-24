use rifle_rules::Catalog;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let program: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/rust-program.json"))?;
    let package = serde_json::to_vec(&serde_json::json!({
        "id": "example.program", "version": "1.0.0", "program": program,
        "description": "Opaque host metadata also contributes to identity"
    }))?;
    let catalog = Catalog::from_json_documents(&[&package])?;
    println!("catalog {}", catalog.digest());
    println!("package {}", catalog.packages()[0].digest());
    Ok(())
}
