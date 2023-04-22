use std::{sync::Arc, time::Duration};

use nxzr_transport::semaphore::BoundedSemaphore;
use tokio::time::sleep;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    let bsem = Arc::new(BoundedSemaphore::new(4, 4));
    let bsem2 = bsem.clone();
    let handle = tokio::spawn(async move {
        let mut join_handles = Vec::new();

        for i in 0..6 {
            let bsem2 = bsem2.clone();
            join_handles.push(tokio::spawn(async move {
                let Ok(_) = bsem2.acquire_forget_owned().await else {
                    println!("Error acquiring semaphore.");
                    return;
                };
                println!("Consuming permit: {}", i + 1);
                sleep(Duration::from_millis(5000)).await;
                println!("Finish processing: {}", i + 1);
            }));
        }

        for handle in join_handles {
            handle.await.unwrap();
        }
    });
    let bsem3 = bsem.clone();
    let handle2 = tokio::spawn(async move {
        sleep(Duration::from_millis(2000)).await;
        println!("Adding a first permit.");
        bsem3.add_permits(1);
        sleep(Duration::from_millis(1000)).await;
        println!("Adding a second permit.");
        bsem3.add_permits(1);
    });
    handle.await.unwrap();
    handle2.await.unwrap();
    Ok(())
}
