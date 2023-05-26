mod external_scripts;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    external_scripts::install_system_requirements().await?;
    Ok(())
}
