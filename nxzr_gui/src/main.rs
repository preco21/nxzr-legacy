mod external_scripts;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup a tracer.
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    // Check whether the program runs with elevated privileges.
    external_scripts::install_system_requirements().await?;
    Ok(())
}

// FIXME: default key binding
// 1 -> left
// 2 -> up
// 3 -> down
// 4 -> right
// tab -> x
// q -> rs
// w -> ls-up
// a -> ls-left
// s -> ls-down
// d -> ls-right
// e -> -
// r -> +
// f -> y
// ctrl -> zl
// alt -> a
// space -> b
// . -> ls
// p -> cap
// up -> rs-up
// left -> rs-left
// down -> rs-down
// right -> rs-right
